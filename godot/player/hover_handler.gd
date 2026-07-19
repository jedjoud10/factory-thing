extends Node3D

var rigidbody_hover_target: RigidBody3D = null
@onready var camera = $"../Camera3D"
@onready var raycaster = $"../Camera3D/RayCast3D"

@export var hover_force = 2000
@export var hover_damp_force = 100
@export var hover_forward_factor = 3
	
func _physics_process(delta: float) -> void:
	if (Input.is_action_just_pressed("do_hover_thing")):
		if (raycaster.is_colliding()):
			var node = raycaster.get_collider()
			
			if (rigidbody_hover_target != null):
				rigidbody_hover_target = null
			elif (node is RigidBody3D):
				rigidbody_hover_target = node
				rigidbody_hover_target.freeze = false
		else:
			rigidbody_hover_target = null
		
	if (rigidbody_hover_target != null):
		var target_position = camera.global_position - camera.global_basis.z * hover_forward_factor
		var current_position = rigidbody_hover_target.global_position
		var force = (target_position - current_position) * delta * hover_force
		force += -rigidbody_hover_target.linear_velocity * delta * hover_damp_force
		rigidbody_hover_target.apply_force(force)
