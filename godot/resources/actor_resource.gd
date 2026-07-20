class_name ActorResource
extends Resource

@export var scene: PackedScene
@export var type: ActorType

enum ActorType {
	Machine,
	PowerPole,
	Wire,
	Belt,
	Silo
}

# Make sure that every parameter has a default value.
# Otherwise, there will be problems with creating and editing
# your resource via the inspector.
func _init(p_scene: PackedScene = null, p_type: ActorType = ActorType.Machine):
	type = p_type
	scene = p_scene
