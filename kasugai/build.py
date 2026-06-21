import subprocess
import os

def build_tauri_app():
    # Change to the directory where package.json and tauri.conf.json are located
    # This assumes the script is run from the project root or the current working directory is set correctly
    project_root = os.path.join(os.path.dirname(__file__))
    command = ["cmd.exe", "/c", "npx", "tauri", "build"]
    
    print(f"Running command: {' '.join(command)} in {project_root}")
    try:
        process = subprocess.run(command, cwd=project_root, check=True, capture_output=True, text=True)
        print("Build successful!")
        print("stdout:")
        print(process.stdout)
        print("stderr:")
        print(process.stderr)
    except subprocess.CalledProcessError as e:
        print(f"Build failed with error code {e.returncode}")
        print("stdout:")
        print(e.stdout)
        print("stderr:")
        print(e.stderr)
    except FileNotFoundError:
        print("Error: 'npx' command not found. Make sure Node.js and npm are installed and in your PATH.")

if __name__ == "__main__":
    build_tauri_app()