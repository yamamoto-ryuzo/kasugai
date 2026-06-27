import subprocess
import os
import json

def modify_tauri_config(project_root):
    config_path = os.path.join(project_root, 'src-tauri', 'tauri.conf.json')
    
    with open(config_path, 'r', encoding='utf-8') as f:
        config_text = f.read()
    
    # JSON with comments might fail to parse with standard json library.
    # We will do a simple string replacement as a workaround.
    # This is fragile, but the json file seems to be minified and consistent.
    new_config_text = config_text.replace('"targets":"all"', '"targets":[]')
    
    with open(config_path, 'w', encoding='utf-8') as f:
        f.write(new_config_text)
    
    print(f"Modified {config_path} to disable installers.")

def build_tauri_app():
    # Change to the directory where package.json and tauri.conf.json are located
    # This assumes the script is run from the project root or the current working directory is set correctly
    project_root = os.path.join(os.path.dirname(__file__))
    
    # Modify the config before building
    modify_tauri_config(project_root)

    command = ["cmd.exe", "/c", "npx", "tauri", "build"]
    
    print(f"Running command: {' '.join(command)} in {project_root}")
    try:
        process = subprocess.run(command, cwd=project_root, check=True, capture_output=True, text=True, encoding='utf-8')
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
