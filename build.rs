fn main() {
    let frontend_dist = std::path::Path::new("frontend").join("dist");
    if !frontend_dist.join("index.html").exists() {
        println!("cargo:warning=Frontend not built. Run: cd frontend && npm install && npx vite build");
    }
    println!("cargo:rerun-if-changed=frontend/dist/");
}
