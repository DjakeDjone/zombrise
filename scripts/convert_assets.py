import bpy
import os
import sys

# Usage: blender --background --python scripts/convert_assets.py

def convert_assets():
    # Directory setup
    script_dir = os.path.dirname(os.path.abspath(__file__))
    assets_dir = os.path.abspath(os.path.join(script_dir, "assets_to_convert"))
    output_file = os.path.join(assets_dir, "zombie.glb")

    print(f"Scanning {assets_dir} for FBX files...")

    # Clear the scene
    bpy.ops.wm.read_factory_settings(use_empty=True)

    # Find FBX files
    fbx_files = [f for f in os.listdir(assets_dir) if f.lower().endswith(".fbx")]
    fbx_files.sort()

    if not fbx_files:
        print("No FBX files found.")
        return

    # We need to keep track of the main armature object
    main_armature = None

    # Import all FBX files
    for i, fbx in enumerate(fbx_files):
        path = os.path.join(assets_dir, fbx)
        print(f"Importing: {fbx}")
        
        # Import FBX
        # force_connect_children=True helps with bone hierarchy in some cases
        # automatic_bone_orientation=True is usually good for FBX->GLTF
        bpy.ops.import_scene.fbx(filepath=path, automatic_bone_orientation=True)

        # Get the imported objects
        imported_objects = [obj for obj in bpy.context.selected_objects]
        
        # Find the armature in the imported objects
        armature = None
        for obj in imported_objects:
            if obj.type == 'ARMATURE':
                armature = obj
                break
        
        if not armature:
            print(f"Warning: No armature found in {fbx}")
            continue

        # Extract animation name from filename (e.g., "Zombie Walk.fbx" -> "Zombie Walk")
        anim_name = os.path.splitext(fbx)[0]
        
        if armature.animation_data and armature.animation_data.action:
            action = armature.animation_data.action
            action.name = anim_name
            print(f"  Found action: {action.name}")
            
            # If this is the first file, keep it as the main armature
            if i == 0:
                main_armature = armature
                # Ensure the action is stashed/kept
                if not main_armature.animation_data:
                    main_armature.animation_data_create()
                
                # Push the action to NLA track to ensure it's exported
                track = main_armature.animation_data.nla_tracks.new()
                track.name = action.name
                track.strips.new(action.name, int(action.frame_range[0]), action)
                
            else:
                # For subsequent files, we just want the action
                # We can delete the imported objects, but keep the action
                # The action is already in bpy.data.actions
                
                # Assign the action to the main armature to ensure compatibility (optional but good check)
                # And push it to the main armature's NLA tracks
                if main_armature:
                    if not main_armature.animation_data:
                        main_armature.animation_data_create()
                        
                    track = main_armature.animation_data.nla_tracks.new()
                    track.name = action.name
                    try:
                        track.strips.new(action.name, int(action.frame_range[0]), action)
                    except Exception as e:
                        print(f"  Error adding NLA strip for {action.name}: {e}")

                # Delete the imported objects from this file (except the action)
                bpy.ops.object.delete()
        else:
            print(f"  No animation found in {fbx}")
            # If it's the first file and has no animation, we still keep it as the base mesh
            if i == 0:
                main_armature = armature

    # Select the main armature and its children (mesh) for export
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
            # Ensure we export all NLA tracks as animations
            export_nla_strips=True, 
            # export_animations=True is default
        )
        print("Conversion complete.")
    else:
        print("Error: No main armature found.")

if __name__ == "__main__":
    convert_assets()
