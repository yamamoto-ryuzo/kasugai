import os
import sys
import subprocess
import shutil
import json
from datetime import datetime

def main():
    # スクリプトの格納ディレクトリを取得 (c:\github\kasugai - プロジェクトルート)
    script_dir = os.path.dirname(os.path.abspath(__file__))
    
    # Tauriプロジェクトが存在するサブディレクトリ (c:\github\kasugai\kasugai)
    target_dir = os.path.join(script_dir, "kasugai")
    
    if not os.path.exists(target_dir):
        print(f"エラー: Tauriプロジェクトディレクトリが見つかりません: {target_dir}")
        sys.exit(1)
        
    # ターゲットディレクトリへ移動
    print(f"[Kasugai] カレントディレクトリを移動中: {target_dir}")
    os.chdir(target_dir)
    
    # コマンド引数の解析
    # 引数がない、または "dev" の場合は開発起動
    # "build" の場合は本番用ビルドを実行
    mode = "dev"
    if len(sys.argv) > 1:
        arg = sys.argv[1].lower()
        if arg in ["build", "b"]:
            mode = "build"
        elif arg in ["dev", "d"]:
            mode = "dev"
        else:
            print(f"未知の引数: {sys.argv[1]}")
            print("使用法: python run.py [dev|build]")
            sys.exit(1)

    # 必要なコマンドが存在するか確認 (npx)
    if not shutil.which("npx"):
        print("エラー: 'npx' コマンドが見つかりません。Node.js がインストールされているか確認してください。")
        sys.exit(1)

    # Tauri コマンドの組み立て
    # Windowsシステムを考慮し shell=True を指定します
    tauri_cmd = ["npx", "tauri", mode]
    
    print(f"[Kasugai] Tauri {mode} モードを起動します...")
    print(f"実行コマンド: {' '.join(tauri_cmd)}")
    
    try:
        # コマンドの実行 (標準入出力を引き継ぐ)
        result = subprocess.run(tauri_cmd, shell=True, check=True)
        # ビルドモードの場合、生成された EXE を download フォルダへコピーし、配信用ファイルを作成
        if mode == "build":
            try:
                bundle_dir = os.path.join(target_dir, 'src-tauri', 'target', 'release', 'bundle')
                download_dir = os.path.join(script_dir, 'download')
                os.makedirs(download_dir, exist_ok=True)
                dest_json = os.path.join(download_dir, 'latest.json')

                # Tauri v2 updater 用インストーラーを検索（NSIS を優先、次に MSI）
                installer_src = None
                sig_path = None
                dest_installer = None
                bundle_candidates = [
                    ('nsis', '.exe', 'kasugai.exe'),
                    ('msi', '.msi', 'kasugai.msi'),
                ]
                for kind, installer_ext, dest_name in bundle_candidates:
                    subdir = os.path.join(bundle_dir, kind)
                    if not os.path.isdir(subdir):
                        continue
                    installer_files = [
                        f for f in os.listdir(subdir)
                        if f.endswith(installer_ext) and not f.endswith('.sig')
                    ]
                    if installer_files:
                        installer_src = os.path.join(subdir, installer_files[0])
                        sig_path = installer_src + '.sig'
                        dest_installer = os.path.join(download_dir, dest_name)
                        print(f"[Kasugai] {kind.upper()} インストーラーを見つけました: {installer_src}")
                        break

                if not installer_src:
                    print("[Kasugai] エラー: インストーラー（.exe または .msi）が見つかりません")
                    sys.exit(1)

                # インストーラーを download フォルダへコピー
                shutil.copy2(installer_src, dest_installer)
                print(f"[Kasugai] インストーラーをコピーしました: {dest_installer}")

                # 署名ファイルを読み込み
                signature = ""
                if sig_path and os.path.exists(sig_path):
                    with open(sig_path, 'r', encoding='utf-8') as f:
                        signature = f.read().strip()
                    print(f"[Kasugai] 署名ファイルを読み込みました: {sig_path}")
                else:
                    print("[Kasugai] 警告: 署名ファイルが見つかりません。latest.json の signature が空になります。")

                # バージョン情報を取得
                tauri_conf_path = os.path.join(target_dir, 'src-tauri', 'tauri.conf.json')
                version = "1.2.0"  # デフォルトバージョン
                if os.path.exists(tauri_conf_path):
                    with open(tauri_conf_path, 'r', encoding='utf-8') as f:
                        conf = json.load(f)
                        version = conf.get('version', '1.2.0')

                # 更新JSONを生成
                print(f"[Kasugai] 更新JSONを生成中: {dest_json}")
                update_data = {
                    "version": version,
                    "notes": f"Kasugai バージョン {version}\n\n- 最新の更新内容",
                    "pub_date": datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ"),
                    "platforms": {
                        "windows-x86_64": {
                            "signature": signature,
                            "url": f"https://yamamoto-ryuzo.github.io/kasugai/download/{os.path.basename(dest_installer)}"
                        }
                    }
                }

                with open(dest_json, 'w', encoding='utf-8') as f:
                    json.dump(update_data, f, indent=2, ensure_ascii=False)
                print(f"[Kasugai] 更新JSONを生成しました: {dest_json}")

                print(f"[Kasugai] 配信用ファイルの準備が完了しました")
                print(f"[Kasugai] - INSTALLER: {dest_installer}")
                print(f"[Kasugai] - JSON: {dest_json}")

            except Exception as e:
                print(f"[Kasugai] 配信用ファイル作成中にエラー: {e}")
                import traceback
                traceback.print_exc()

        sys.exit(result.returncode)
    except subprocess.CalledProcessError as e:
        print(f"\n[Kasugai] エラー: コマンドの実行に失敗しました。終了コード: {e.returncode}")
        sys.exit(e.returncode)
    except KeyboardInterrupt:
        print("\n[Kasugai] プロセスはユーザーによって中断されました。")
        sys.exit(0)

if __name__ == "__main__":
    main()
