use axum::{
    Json, Router,
    extract::Query,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() {
    let dist = format!(
        "{}/frontend/vite-monorepo/apps/web/dist",
        env!("CARGO_MANIFEST_DIR")
    );
    println!("DIST = {dist}");
    println!("DIST 存在: {}", Path::new(&dist).is_dir());
    let index = format!("{dist}/index.html");

    let 路由 = Router::new()
        .route("/api/check-wallpaper", get(检查桌面壁纸))
        .route("/api/set-wallpaper", post(设置桌面壁纸接口))
        .route("/api/pick-wallpaper", get(选择壁纸文件))
        .route("/api/pick-and-set", get(选择并设置壁纸))
        .layer(CorsLayer::permissive())
        .fallback_service(ServeDir::new(dist).not_found_service(ServeFile::new(index)));

    let 监听器 = tokio::net::TcpListener::bind("0.0.0.0:4242").await.unwrap();
    println!("运行在：http://127.0.0.1:4242");
    axum::serve(监听器, 路由).await.unwrap();
}

#[derive(Deserialize)]
struct 检查请求 {
    path: String,
}

#[derive(Serialize)]
struct 结果 {
    ok: bool,
    message: String,
}

#[derive(Deserialize)]
struct 设置请求 {
    path: String,
}

// ---------------- handlers ----------------

async fn 检查桌面壁纸(Query(q): Query<检查请求>) -> Json<结果> {
    let p = PathBuf::from(&q.path);
    let 是文件 = tokio::fs::metadata(&p)
        .await
        .map(|m| m.is_file())
        .unwrap_or(false);
    Json(结果 {
        ok: 是文件,
        message: if 是文件 {
            format!("存在: {}", p.display())
        } else {
            format!("不存在: {}", p.display())
        },
    })
}

async fn 选择壁纸文件() -> Json<结果> {
    let path = tokio::task::spawn_blocking(|| {
        rfd::FileDialog::new()
            .add_filter("图片", &["png", "jpg", "jpeg", "bmp", "webp", "gif"])
            .set_title("选择壁纸")
            .pick_file()
    })
    .await
    .unwrap();

    Json(match path {
        Some(p) => 结果 {
            ok: true,
            message: p.to_string_lossy().into_owned(),
        },
        None => 结果 {
            ok: false,
            message: "用户取消".into(),
        },
    })
}

async fn 选择并设置壁纸() -> Json<结果> {
    let path = tokio::task::spawn_blocking(|| {
        rfd::FileDialog::new()
            .add_filter("图片", &["png", "jpg", "jpeg", "bmp", "webp"])
            .set_title("选择壁纸")
            .pick_file()
    })
    .await
    .unwrap();

    let Some(p) = path else {
        return Json(结果 {
            ok: false,
            message: "用户取消".into(),
        });
    };

    let p_str = p.to_string_lossy().into_owned();
    let 设置结果: Result<(), String> = tokio::task::spawn_blocking(move || 设置壁纸(&p_str))
        .await
        .unwrap();

    Json(match 设置结果 {
        Ok(_) => 结果 {
            ok: true,
            message: format!("已设置: {}", p.display()),
        },
        Err(e) => 结果 {
            ok: false,
            message: format!("设置失败: {e}"),
        },
    })
}

async fn 设置桌面壁纸接口(Json(请求): Json<设置请求>) -> Json<结果> {
    let p = PathBuf::from(&请求.path);

    if tokio::fs::metadata(&p)
        .await
        .map(|m| !m.is_file())
        .unwrap_or(true)
    {
        return Json(结果 {
            ok: false,
            message: format!("文件不存在: {}", p.display()),
        });
    }

    let path_str = 请求.path.clone();
    let 输出: Result<(), String> = tokio::task::spawn_blocking(move || 设置壁纸(&path_str))
        .await
        .unwrap();

    Json(match 输出 {
        Ok(_) => 结果 {
            ok: true,
            message: "设置成功".into(),
        },
        Err(e) => 结果 {
            ok: false,
            message: format!("设置失败: {e}"),
        },
    })
}

// ---------------- 跨平台设置壁纸 ----------------

/// Linux 下自动适配 KDE/GNOME，其它平台回退到 wallpaper 库
fn 设置壁纸(path: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_lowercase();
        let uri = format!("file://{}", path);

        // 方案 A：KDE Plasma，优先 plasma-apply-wallpaperimage
        if desktop.contains("kde") || desktop.contains("plasma") {
            // A1
            if let Ok(out) = Command::new("plasma-apply-wallpaperimage")
                .arg(path)
                .output()
            {
                if out.status.success() {
                    return Ok(());
                }
                eprintln!(
                    "[设置壁纸] plasma-apply-wallpaperimage 失败: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
            } else {
                eprintln!("[设置壁纸] plasma-apply-wallpaperimage 未安装，回退 qdbus6");
            }

            // A2：qdbus6 脚本兜底
            let script = format!(
                r#"var allDesktops = desktops();
                for (i=0; i<allDesktops.length; i++) {{
                    d = allDesktops[i];
                    d.wallpaperPlugin = "org.kde.image";
                    d.currentConfigGroup = Array("Wallpaper", "org.kde.image", "General");
                    d.writeConfig("Image", "{}");
                }}"#,
                uri
            );

            let out = Command::new("qdbus6")
                .args([
                    "org.kde.plasmashell",
                    "/PlasmaShell",
                    "org.kde.PlasmaShell.evaluateScript",
                    &script,
                ])
                .output()
                .map_err(|e| format!("qdbus6 不可用: {e}"))?;

            if !out.status.success() {
                return Err(format!(
                    "qdbus6 执行失败: {}",
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
            return Ok(());
        }

        // 方案 B：GNOME / Unity / Cinnamon
        if desktop.contains("gnome") || desktop.contains("unity") || desktop.contains("cinnamon") {
            let out = Command::new("gsettings")
                .args(["set", "org.gnome.desktop.background", "picture-uri", &uri])
                .output()
                .map_err(|e| format!("gsettings 不可用: {e}"))?;

            let _ = Command::new("gsettings")
                .args([
                    "set",
                    "org.gnome.desktop.background",
                    "picture-uri-dark",
                    &uri,
                ])
                .output();

            if !out.status.success() {
                return Err(format!(
                    "gsettings 执行失败: {}",
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
            return Ok(());
        }

        Err(format!("暂不支持的桌面环境: '{}'", desktop))
    }

    #[cfg(not(target_os = "linux"))]
    {
        wallpaper::set_from_path(path).map_err(|e| e.to_string())
    }
}
