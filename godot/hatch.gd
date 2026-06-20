extends Node3D

enum HatchType {
	Input,
	Output,
}

@export var type: HatchType

@onready var box = $CSGBox3D

func _ready() -> void:
	box.material=box.material.duplicate()
	if (type == HatchType.Input):
		box.material.albedo_color = Color.BLACK
	else:
		box.material.albedo_color = Color.WHITE
	
	pass # Replace with function body.
