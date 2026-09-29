//! Building the game command line and supervising the running game process.

use crate::auth;
use crate::bail;
use crate::download::emit_stage;
use crate::error::{Error, Result};
use crate::install;
use crate::java;
use crate::models::{GameState, Instance, LogLine, StoredAccount};
use crate::state::{now_secs, BusyGuard, Shared};
use crate::versions::{rules_allow, Argument, VersionJson};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::oneshot;

pub const LAUNCHER_NAME: &str = "NebulaLauncher";

pub fn emit_state(app: &AppHandle, id: &str, state: &str, message: Option<String>, exit_code: Option<i32>) {
    let _ = app.emit(
        "game-state",
        GameState {
            instance_id: id.to_string(),
            state: state.to_string(),
            message,
            exit_code,
        },
    );
}

pub fn log(app: &AppHandle, st: &Shared, id: &str, stream: &str, line: impl Into<String>) {
    let entry = LogLine {
        instance_id: id.to_string(),
        stream: stream.to_string(),
        line: line.into(),
    };
    st.push_log(entry.clone());
    let _ = app.emit("game-log", entry);
}

fn subst(input: &str, vars: &HashMap<String, String>) -> String {
    let mut out = input.to_string();
    if !out.contains("${") {
        return out;
    }
    for (k, v) in vars {
        let needle = format!("${{{k}}}");
        if out.contains(&needle) {
            out = out.replace(&needle, v);
        }
    }
    out
}

fn eval_args(
    list: &[Argument],
    features: &HashMap<String, bool>,
    vars: &HashMap<String, String>,
) -> Vec<String> {
    let mut out = Vec::new();
    for a in list {
        match a {
            Argument::Plain(s) => out.push(subst(s, vars)),
            Argument::Conditional { rules, value } => {
                if rules_allow(rules, features) {
                    for v in value.to_vec() {
                        out.push(subst(&v, vars));
                    }
                }
            }
        }
    }
    out
}

async fn resolve_java(
    app: &AppHandle,
    st: &Shared,
    id: &str,
    inst: &Instance,
    vj: &VersionJson,
) -> Result<String> {
    if !inst.java_path.trim().is_empty() {
        return Ok(inst.java_path.trim().to_string());
    }
    let (global, managed) = st.read(|s| (s.settings.java_path.clone(), s.settings.managed_java));
    if !global.trim().is_empty() {
        return Ok(global.trim().to_string());
    }
    if managed {
        let component = vj.java_component();
        match java::ensure_runtime(app, st, id, &component).await {
            Ok(Some(p)) => return Ok(p.to_string_lossy().to_string()),
            Ok(None) => log(
                app,
                st,
                id,
                "launcher",
                format!("No managed Java runtime ({component}) is available for this platform, using system Java."),
            ),
            Err(e) => log(
                app,
                st,
                id,
                "launcher",
                format!("Could not download the Java runtime ({e}). Falling back to system Java."),
            ),
        }
    }
    Ok(java::system_java())
}

/// Validate and kick off a launch in the background. Progress is reported through events.
pub fn start(app: AppHandle, st: Shared, instance_id: String) -> Result<()> {
    let inst = st
        .read(|s| s.instances.iter().find(|i| i.id == instance_id).cloned())
        .ok_or_else(|| Error::msg("Instance not found"))?;
    let acc = st
        .read(|s| {
            s.selected_account
                .as_ref()
                .and_then(|id| s.accounts.iter().find(|a| &a.id == id))
                .or_else(|| s.accounts.first())
                .cloned()
        })
        .ok_or_else(|| Error::msg("Add an account first (Accounts tab)"))?;
    if st.running.lock().unwrap().contains_key(&instance_id) {
        bail!("This instance is already running");
    }
    let guard = BusyGuard::acquire(&st, &instance_id)
        .ok_or_else(|| Error::msg("This instance is already being prepared"))?;

    st.logs.lock().unwrap().remove(&instance_id);
    emit_state(&app, &instance_id, "preparing", None, None);

    tauri::async_runtime::spawn(async move {
        let _guard = guard;
        if let Err(e) = run(&app, &st, inst.clone(), acc).await {
            log(&app, &st, &inst.id, "launcher", format!("Launch failed: {e}"));
            emit_state(&app, &inst.id, "error", Some(e.to_string()), None);
        }
    });
    Ok(())
}

async fn run(app: &AppHandle, st: &Shared, inst: Instance, acc: StoredAccount) -> Result<()> {
    let id = inst.id.clone();

    // 1. Account
    let acc = if acc.kind == "elyby" {
        emit_stage(app, &id, "Checking Ely.by session");
        let updated = auth::elyby_ensure_valid(st, &acc).await?;
        if updated.access_token != acc.access_token || updated.username != acc.username {
            let u = updated.clone();
            st.mutate(|s| {
                if let Some(a) = s.accounts.iter_mut().find(|a| a.id == u.id) {
                    *a = u;
                }
            })?;
        }
        updated
    } else {
        acc
    };

    // 2. Game files
    let vj = install::ensure_version(app, st, &id, &inst.version_id).await?;

    // 3. Java + authlib-injector
    let java_path = resolve_java(app, st, &id, &inst, &vj).await?;
    let injector = if acc.kind == "elyby" {
        Some(auth::ensure_authlib_injector(app, st, &id).await?)
    } else {
        None
    };

    emit_stage(app, &id, "Starting game");

    // 4. Directories and natives
    let game_dir = st.dirs.instance(&id);
    tokio::fs::create_dir_all(&game_dir).await?;
    let libs_dir = st.dirs.libraries();
    let assets_dir = st.dirs.assets();
    let resolved = install::resolve_all(&vj, &libs_dir);

    let mut classpath: Vec<String> = Vec::new();
    for r in &resolved {
        if let Some(a) = &r.artifact {
            let p = a.path.to_string_lossy().to_string();
            if !classpath.contains(&p) {
                classpath.push(p);
            }
        }
    }
    classpath.push(install::client_jar(st, &inst.version_id).to_string_lossy().to_string());

    let natives_dir = st.dirs.natives(&id);
    let native_jars: Vec<(PathBuf, Vec<String>)> = resolved
        .iter()
        .filter_map(|r| r.native.as_ref().map(|n| (n.path.clone(), r.exclude.clone())))
        .collect();
    {
        let natives_dir = natives_dir.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let _ = std::fs::remove_dir_all(&natives_dir);
            std::fs::create_dir_all(&natives_dir)?;
            for (jar, exclude) in native_jars {
                install::extract_natives(&jar, &natives_dir, &exclude)?;
            }
            Ok(())
        })
        .await
        .map_err(|e| Error::msg(e.to_string()))??;
    }

    let asset_index_id = vj
        .asset_index
        .as_ref()
        .map(|a| a.id.clone())
        .or_else(|| vj.assets.clone())
        .unwrap_or_else(|| "legacy".to_string());
    let legacy_assets = install::prepare_legacy_assets(st, &asset_index_id, &game_dir).await?;
    let game_assets = legacy_assets
        .clone()
        .unwrap_or_else(|| assets_dir.clone())
        .to_string_lossy()
        .to_string();

    // 5. Variables & features
    let sep = if cfg!(windows) { ";" } else { ":" };
    let classpath_str = classpath.join(sep);
    let (is_ely, token, user_type) = if acc.kind == "elyby" {
        (true, acc.access_token.clone(), "mojang")
    } else {
        (false, "0".to_string(), "legacy")
    };
    let mut vars: HashMap<String, String> = HashMap::new();
    let mut put = |k: &str, v: String| {
        vars.insert(k.to_string(), v);
    };
    put("auth_player_name", acc.username.clone());
    put("version_name", inst.version_id.clone());
    put("game_directory", game_dir.to_string_lossy().to_string());
    put("assets_root", assets_dir.to_string_lossy().to_string());
    put("assets_index_name", asset_index_id.clone());
    put("auth_uuid", acc.uuid.clone());
    put("auth_access_token", token.clone());
    put("auth_session", if is_ely { format!("token:{}:{}", token, acc.uuid) } else { "-".into() });
    put("clientid", "0".into());
    put("auth_xuid", "0".into());
    put("user_type", user_type.into());
    put("version_type", if vj.kind.is_empty() { "release".into() } else { vj.kind.clone() });
    put("user_properties", "{}".into());
    put("game_assets", game_assets);
    put("natives_directory", natives_dir.to_string_lossy().to_string());
    put("launcher_name", LAUNCHER_NAME.into());
    put("launcher_version", env!("CARGO_PKG_VERSION").into());
    put("classpath", classpath_str.clone());
    put("classpath_separator", sep.into());
    put("library_directory", libs_dir.to_string_lossy().to_string());
    let custom_res = inst.width.is_some() && inst.height.is_some();
    put("resolution_width", inst.width.unwrap_or(854).to_string());
    put("resolution_height", inst.height.unwrap_or(480).to_string());
    let server = inst.server.trim().to_string();
    put("quickPlayMultiplayer", server.clone());

    let mut features: HashMap<String, bool> = HashMap::new();
    features.insert("has_custom_resolution".into(), custom_res);
    features.insert("is_demo_user".into(), false);
    features.insert("is_quick_play_multiplayer".into(), !server.is_empty());

    // 6. JVM arguments
    let (default_mem, global_jvm) = st.read(|s| (s.settings.default_memory_mb, s.settings.jvm_args.clone()));
    let mem = inst.memory_mb.unwrap_or(default_mem).max(256);
    let mut jvm: Vec<String> = vec![format!("-Xmx{mem}M")];
    if mem >= 1024 {
        jvm.push("-Xms512M".to_string());
    }
    if let Some(inj) = &injector {
        jvm.push(format!("-javaagent:{}={}", inj.to_string_lossy(), auth::ELY_INJECTOR_TARGET));
    }
    jvm.extend(global_jvm.split_whitespace().map(String::from));
    jvm.extend(inst.jvm_args.split_whitespace().map(String::from));

    let mut version_jvm = match vj.arguments.as_ref() {
        Some(a) => eval_args(&a.jvm, &features, &vars),
        None => Vec::new(),
    };
    if !version_jvm.iter().any(|a| a.starts_with("-Djava.library.path")) {
        version_jvm.push(format!("-Djava.library.path={}", natives_dir.to_string_lossy()));
    }
    if !version_jvm.iter().any(|a| a == "-cp" || a == "-classpath") {
        version_jvm.push("-cp".into());
        version_jvm.push(classpath_str.clone());
    }
    if !version_jvm.iter().any(|a| a.starts_with("-Dminecraft.launcher.brand")) {
        version_jvm.push(format!("-Dminecraft.launcher.brand={LAUNCHER_NAME}"));
    }
    jvm.extend(version_jvm);

    if let Some(file) = vj
        .logging
        .as_ref()
        .and_then(|l| l.client.as_ref())
        .and_then(|c| c.file.as_ref().map(|f| (c.argument.clone(), f.id.clone())))
    {
        let (arg, file_id) = file;
        let path = install::log_config_path(st, &file_id);
        if !arg.is_empty() && path.is_file() {
            jvm.push(arg.replace("${path}", &path.to_string_lossy()));
        }
    }

    // 7. Game arguments
    let mut game: Vec<String> = match (&vj.arguments, &vj.minecraft_arguments) {
        (Some(a), _) if !a.game.is_empty() => eval_args(&a.game, &features, &vars),
        (_, Some(m)) => m.split_whitespace().map(|s| subst(s, &vars)).collect(),
        _ => Vec::new(),
    };
    let modern_args = vj.arguments.as_ref().map(|a| !a.game.is_empty()).unwrap_or(false);
    if !modern_args && custom_res {
        game.push("--width".into());
        game.push(inst.width.unwrap_or(854).to_string());
        game.push("--height".into());
        game.push(inst.height.unwrap_or(480).to_string());
    }
    if inst.fullscreen {
        game.push("--fullscreen".into());
    }
    if !server.is_empty() && !game.iter().any(|a| a == "--quickPlayMultiplayer") {
        let (host, port) = match server.rsplit_once(':') {
            Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() => (h.to_string(), p.to_string()),
            _ => (server.clone(), "25565".to_string()),
        };
        game.push("--server".into());
        game.push(host);
        game.push("--port".into());
        game.push(port);
    }

    // 8. Spawn
    let mut cmd = tokio::process::Command::new(&java_path);
    cmd.args(&jvm)
        .arg(&vj.main_class)
        .args(&game)
        .current_dir(&game_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        cmd.creation_flags(0x0800_0000);
    }

    let mask = |s: &str| -> String {
        if is_ely && !token.is_empty() {
            s.replace(&token, "***")
        } else {
            s.to_string()
        }
    };
    log(app, st, &id, "launcher", format!("Minecraft {} as {} ({})", inst.version_id, acc.username, acc.kind));
    log(app, st, &id, "launcher", format!("Java: {java_path}"));
    log(
        app,
        st,
        &id,
        "launcher",
        mask(&format!("Command: {} {} {} {}", java_path, jvm.join(" "), vj.main_class, game.join(" "))),
    );

    let mut child = cmd
        .spawn()
        .map_err(|e| Error::msg(format!("Could not start Java (`{java_path}`): {e}. Install Java or set a Java path in Settings.")))?;

    let (kill_tx, kill_rx) = oneshot::channel::<()>();
    st.running.lock().unwrap().insert(id.clone(), kill_tx);
    emit_state(app, &id, "running", None, None);

    let hide = st.read(|s| s.settings.hide_on_launch);
    if hide {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.hide();
        }
    }

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let app2 = app.clone();
    let st2 = st.clone();
    let started = now_secs();
    tauri::async_runtime::spawn(async move {
        let mut pumps = Vec::new();
        if let Some(o) = stdout {
            pumps.push(tauri::async_runtime::spawn(pump(app2.clone(), st2.clone(), id.clone(), "stdout", o)));
        }
        if let Some(e) = stderr {
            pumps.push(tauri::async_runtime::spawn(pump(app2.clone(), st2.clone(), id.clone(), "stderr", e)));
        }

        let mut killed = false;
        let status = tokio::select! {
            s = child.wait() => s.ok(),
            _ = kill_rx => {
                killed = true;
                let _ = child.kill().await;
                child.wait().await.ok()
            }
        };
        for p in pumps {
            let _ = tokio::time::timeout(Duration::from_secs(2), p).await;
        }

        st2.running.lock().unwrap().remove(&id);
        let code = status.and_then(|s| s.code());
        let played = now_secs().saturating_sub(started);
        let id_c = id.clone();
        let _ = st2.mutate(|s| {
            if let Some(i) = s.instances.iter_mut().find(|i| i.id == id_c) {
                i.last_played = Some(started);
                i.play_time_secs += played;
            }
        });

        let (state, msg) = if killed || code == Some(0) {
            ("stopped", None)
        } else {
            (
                "crashed",
                Some(format!("The game exited with code {}", code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".into()))),
            )
        };
        log(
            &app2,
            &st2,
            &id,
            "launcher",
            format!("Game exited{}", code.map(|c| format!(" with code {c}")).unwrap_or_default()),
        );
        emit_state(&app2, &id, state, msg, code);
        let _ = std::fs::remove_dir_all(st2.dirs.natives(&id));
        if hide {
            if let Some(w) = app2.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
    });

    Ok(())
}

async fn pump<R: AsyncRead + Unpin>(app: AppHandle, st: Shared, id: String, stream: &'static str, reader: R) {
    let mut reader = BufReader::new(reader);
    let mut buf: Vec<u8> = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let line = String::from_utf8_lossy(&buf).trim_end_matches(['\r', '\n']).to_string();
                log(&app, &st, &id, stream, line);
            }
        }
    }
}

/// Ask a running game to terminate.
pub fn kill(st: &Shared, instance_id: &str) -> bool {
    let tx = st.running.lock().unwrap().remove(instance_id);
    match tx {
        Some(tx) => tx.send(()).is_ok(),
        None => false,
    }
}
