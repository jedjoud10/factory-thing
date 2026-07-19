extends Node3D

@onready var indicator = $"../Select Indicator"
@onready var camera = $"../Camera3D"
@onready var raycaster = $"../Camera3D/RayCast3D"

# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta: float) -> void:
	if (raycaster.get_collider() == null):
		indicator.hide()
	else:
		indicator.show()
		indicator.global_position = raycaster.get_collision_point()
