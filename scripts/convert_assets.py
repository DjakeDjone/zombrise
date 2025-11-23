import bpy
import os
import sys
import subprocess
import shutil

# Usage: blender --background --python scripts/convert_assets.py

def convert_assets():
    # Directory setup
    # This script is expected to be in scripts/
    script_dir = os.path.dirname(os.path.abspath(__file__))
    # Assets are in ../assets/
    assets_dir = os.path.abspath(os.path.join(script_dir, "./assets"))
    output_file = os.path.join(assets_dir, "character.glb")
    
    # Path to the external converter tool
    converter_tool = os.path.abspath(os.path.join(script_dir, "../tools/fbx-converter/convert.py"))

    print(f"Scanning {assets_dir} for FBX files...")

    # Clear the scene
    bpy.ops.wm.read_factory_settings(use_empty=True)

    # Find FBX files
    fbx_files = [f for f in os.listdir(assets_dir) if f.lower().endswith(".fbx")]
    fbx_files.sort()

    if not fbx_files:
        print("No FBX files found.")
        return

    # Import all FBX files
    for fbx in fbx_files:
        path = os.path.join(assets_dir, fbx)
        print(f"Importing: {fbx}")
        
        try:
            # Automatic bone orientation usually works best for FBX -> GLTF pipelines
            bpy.ops.import_scene.fbx(filepath=path, automatic_bone_orientation=True)
        except RuntimeError as e:
            print(f"Error importing {fbx}: {e}")
            if "Version 6100 unsupported" in str(e):
                print(f"Attempting to convert legacy FBX file: {fbx}")
                try:
                    # Run the external converter
                    # We use the system python3 to run the converter script
                    subprocess.run(["python3", converter_tool, path], check=True)
                    
                    # The converter outputs to /tmp/fbx_convert_temp.fbx (hardcoded in convert.py)
                    converted_temp = "/tmp/fbx_convert_temp.fbx"
                    
                    if os.path.exists(converted_temp):
                        print(f"Conversion successful. Overwriting original file: {path}")
                        shutil.copy(converted_temp, path)
                        
                        # Retry import
                        print(f"Retrying import for: {fbx}")
                        bpy.ops.import_scene.fbx(filepath=path, automatic_bone_orientation=True)
                    else:
                        print("Conversion failed: Output file not found.")
                except Exception as conv_err:
                    print(f"Failed to convert {fbx}: {conv_err}")
            else:
                print(f"Skipping {fbx} due to error.")
            continue

    # Export to GLB
    print(f"Exporting to: {output_file}")
    bpy.ops.export_scene.gltf(
        filepath=output_file,
        export_format='GLB',
        export_yup=True,
    )
    print("Conversion complete.")

if __name__ == "__main__":
    convert_assets()

