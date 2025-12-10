import bpy
import os
import sys

# Usage: blender --background --python scripts/convert_assets.py

def convert_assets():
    # Setup dirs
    script_dir = os.path.dirname(os.path.abspath(__file__))
    assets_dir = os.path.abspath(os.path.join(script_dir, "assets_to_convert"))
    output_file = os.path.join(assets_dir, "zombie.glb")

    print(f"Scanning {assets_dir} for FBX files...")

    # Clear scene
    bpy.ops.wm.read_factory_settings(use_empty=True)

    # Find FBXs
    fbx_files = [f for f in os.listdir(assets_dir) if f.lower().endswith(".fbx")]
    fbx_files.sort()

    if not fbx_files:
        print("No FBX files found.")
        return

    # Track main armature
    main_armature = None

    # Import FBXs
    for i, fbx in enumerate(fbx_files):
        path = os.path.join(assets_dir, fbx)
        print(f"Importing: {fbx}")
        
        # Import FBX
        # force_connect useful
        # auto_bone good for GLTF
        bpy.ops.import_scene.fbx(filepath=path, automatic_bone_orientation=True)

        # Get the imported objects
        imported_objects = [obj for obj in bpy.context.selected_objects]
        
        # Find armature
        armature = None
        for obj in imported_objects:
            if obj.type == 'ARMATURE':
                armature = obj
                break
        
        if not armature:
            print(f"Warning: No armature found in {fbx}")
            continue

        # Extract anim name
        anim_name = os.path.splitext(fbx)[0]
        
        if armature.animation_data and armature.animation_data.action:
            action = armature.animation_data.action
            action.name = anim_name
            print(f"  Found action: {action.name}")
            
            # First file is main
            if i == 0:
                main_armature = armature
                # Stash action
                if not main_armature.animation_data:
                    main_armature.animation_data_create()
                
                # Push to NLA
                track = main_armature.animation_data.nla_tracks.new()
                track.name = action.name
                track.strips.new(action.name, int(action.frame_range[0]), action)
                
            else:
                # Subsequent files: keep action only

                # Assign action to main
                if main_armature:
                    if not main_armature.animation_data:
                        main_armature.animation_data_create()
                        
                    track = main_armature.animation_data.nla_tracks.new()
                    track.name = action.name
                    try:
                        track.strips.new(action.name, int(action.frame_range[0]), action)
                    except Exception as e:
                        print(f"  Error adding NLA strip for {action.name}: {e}")

                # Delete imported objects
                bpy.ops.object.delete()
        else:
            print(f"  No animation found in {fbx}")
            # First file: keep even if no anim
            if i == 0:
                main_armature = armature

    # Select for export
    if main_armature:
        # Deselect all
        bpy.ops.object.select_all(action='DESELECT')
        
        # Select armature
        main_armature.select_set(True)
        
        # Select children (meshes)
        for child in main_armature.children:
            child.select_set(True)
            
        # Export to GLB
        print(f"Exporting to: {output_file}")
        bpy.ops.export_scene.gltf(
            filepath=output_file,
            export_format='GLB',
            export_yup=True,
            # Export NLA
            export_nla_strips=True, 
            # export_animations=True default
        )
        print("Conversion complete.")
    else:
        print("Error: No main armature found.")

if __name__ == "__main__":
    convert_assets()
