extends Node3D

func _ready() -> void:
	var factory_manager = get_tree().root.get_child(0).get_node("FactoryManager")
	factory_manager.spawn_machine(self)

func _exit_tree() -> void:
	var factory_manager = get_tree().root.get_child(0).get_node("FactoryManager")
	factory_manager.despawn_machine(self)
