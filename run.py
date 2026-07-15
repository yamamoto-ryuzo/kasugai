import os
import sys
import subprocess
import shutil

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
        # ビルドモードの場合、生成された EXE を download フォルダへコピーする
        if mode == "build":
            try:
                # ここでは単純に固定パスをコピーするのみ
                src_exe = os.path.join(target_dir, 'src-tauri', 'target', 'release', 'kasugai.exe')
                download_dir = os.path.join(script_dir, 'download')
                os.makedirs(download_dir, exist_ok=True)
                dest = os.path.join(download_dir, 'kasugai.exe')
                if os.path.exists(src_exe):
                    shutil.copy2(src_exe, dest)
                    print(f"[Kasugai] ビルド生成物をコピーしました: {dest}")
                else:
                    print(f"[Kasugai] 警告: 期待される EXE が見つかりません: {src_exe}")
            except Exception as e:
                print(f"[Kasugai] EXE コピー中にエラー: {e}")

        sys.exit(result.returncode)
    except subprocess.CalledProcessError as e:
        print(f"\n[Kasugai] エラー: コマンドの実行に失敗しました。終了コード: {e.returncode}")
        sys.exit(e.returncode)
    except KeyboardInterrupt:
        print("\n[Kasugai] プロセスはユーザーによって中断されました。")
        sys.exit(0)

if __name__ == "__main__":
    main()
