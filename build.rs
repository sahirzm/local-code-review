fn main() {
    let frontend_dist = std::path::Path::new("frontend").join("dist");
    let index = frontend_dist.join("index.html");

    println!("cargo:rerun-if-changed=frontend/dist/");
    println!("cargo:rerun-if-env-changed=LOCAL_REVIEW_SKIP_FRONTEND_CHECK");

    if std::env::var_os("LOCAL_REVIEW_SKIP_FRONTEND_CHECK").is_some() {
        return;
    }

    if !index.exists() {
        let profile = std::env::var("PROFILE").unwrap_or_default();
        let msg = "frontend/dist/index.html not found. Run: cd frontend && npm install && npx vite build (or set LOCAL_REVIEW_SKIP_FRONTEND_CHECK=1 to bypass)";
        if profile == "release" {
            panic!("{}", msg);
        } else {
            println!("cargo:warning={}", msg);
        }
    }
}
