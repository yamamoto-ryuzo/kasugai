fn main() {
    tauri_build::build();
    // フロントエンドソースを変更した際に自動再ビルドされるように監視対象に追加
    println!("cargo:rerun-if-changed=../src");
}
