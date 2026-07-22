import os
import sys
import subprocess
import shutil
import json
import zipfile
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
                # ファイルパスの設定
                src_exe = os.path.join(target_dir, 'src-tauri', 'target', 'release', 'kasugai.exe')
                download_dir = os.path.join(script_dir, 'download')
                os.makedirs(download_dir, exist_ok=True)
                dest_exe = os.path.join(download_dir, 'kasugai.exe')
                dest_zip = os.path.join(download_dir, 'kasugai.exe.zip')
                dest_json = os.path.join(download_dir, 'latest.json')
                
                # EXEをコピー
                if os.path.exists(src_exe):
                    shutil.copy2(src_exe, dest_exe)
                    print(f"[Kasugai] ビルド生成物をコピーしました: {dest_exe}")
                else:
                    print(f"[Kasugai] 警告: 期待される EXE が見つかりません: {src_exe}")
                    sys.exit(1)
                
                # ZIPファイルを作成
                print(f"[Kasugai] ZIPファイルを作成中: {dest_zip}")
                with zipfile.ZipFile(dest_zip, 'w', zipfile.ZIP_DEFLATED) as zipf:
                    zipf.write(dest_exe, 'kasugai.exe')
                print(f"[Kasugai] ZIPファイルを作成しました: {dest_zip}")
                
                # 署名ファイルを探してコピー
                signature = ""
                sig_dirs = [
                    os.path.join(target_dir, 'src-tauri', 'target', 'release', 'bundle', 'msi'),
                    os.path.join(target_dir, 'src-tauri', 'target', 'release', 'bundle', 'nsis')
                ]
                
                for sig_dir in sig_dirs:
                    if os.path.exists(sig_dir):
                        for file in os.listdir(sig_dir):
                            if file.endswith('.sig'):
                                sig_path = os.path.join(sig_dir, file)
                                with open(sig_path, 'r') as f:
                                    signature = f.read().strip()
                                print(f"[Kasugai] 署名ファイルを見つけました: {sig_path}")
                                break
                    if signature:
                        break
                
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
                            "url": "https://yamamoto-ryuzo.github.io/kasugai/download/kasugai.exe.zip"
                        }
                    }
                }
                
                with open(dest_json, 'w', encoding='utf-8') as f:
                    json.dump(update_data, f, indent=2, ensure_ascii=False)
                print(f"[Kasugai] 更新JSONを生成しました: {dest_json}")
                
                print(f"[Kasugai] 配信用ファイルの準備が完了しました")
                print(f"[Kasugai] - EXE: {dest_exe}")
                print(f"[Kasugai] - ZIP: {dest_zip}")
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
