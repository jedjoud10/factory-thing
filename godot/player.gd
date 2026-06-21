extends CharacterBody3D

@export var sens = 0.001
@export var speed = 5.0
@export var jump_velocity = 4.5
@export var push_force = 0.2

@onready var camera = $"Camera3D"
@onready var raycaster = $"Camera3D/RayCast3D"



var accumulated_mouse = Vector2.ZERO

func _ready() -> void:
	Input.mouse_mode = Input.MOUSE_MODE_CAPTURED

func _physics_process(delta: float) -> void:
	# Add the gravity.
	if not is_on_floor():
		velocity += get_gravity() * delta

	# Handle jump.
	if Input.is_action_just_pressed("player_jump") and is_on_floor():
		velocity.y = jump_velocity

	# Get the input direction and handle the movement/deceleration.
	var input_dir := Input.get_vector("player_move_left", "player_move_right", "player_move_forward", "player_move_backward")
	var direction := (transform.basis * Vector3(input_dir.x, 0, input_dir.y)).normalized()
	if direction:
		velocity.x = direction.x * speed
		velocity.z = direction.z * speed
	else:
		velocity.x = 0
		velocity.z = 0

	move_and_slide()
	
	# after calling move_and_slide()
	for i in get_slide_collision_count():
		var c = get_slide_collision(i)
		if c.get_collider() is RigidBody3D:
			c.get_collider().apply_central_impulse(-c.get_normal() * push_force)
	

func _process(delta: float) -> void:
	var x = clamp(-accumulated_mouse.y, -PI * 0.5, PI * 0.5)
	var y = -accumulated_mouse.x;
	
	camera.rotation = Vector3(x, 0, 0)
	
	# TODO: there's still jittery rotation happening here, probably because of player rotation interpolation
	rotation = Vector3(0, y, 0)
	
	pass

enum ActorType {
	Machine,
	PowerPole,
	Wire,
}


var selected_actor_type: ActorType = ActorType.Machine

var fst_selected_actor: Node3D = null
var snd_selected_actor: Node3D = null

func get_looking_at_actor() -> Node3D:
	if (raycaster.get_collider() != null):
		var parent = raycaster.get_collider().get_parent()
		if (parent.is_in_group("actors")):
			print("select actor ", parent)
			return parent
	return null
	
func _input(event):
	if event.is_action_pressed("ui_cancel"):
		Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
	elif event.is_action_pressed("place_actor"):
		place_actor()
	elif event.is_action_pressed("remove_actor"):
		remove_actor()
	elif event.is_action_pressed("select_machine_as_actor"):
		print("select machine as actor")
		selected_actor_type = ActorType.Machine
	elif event.is_action_pressed("select_pole_as_actor"):
		print("select pole as actor")
		selected_actor_type = ActorType.PowerPole	
	elif event.is_action_pressed("select_wire_as_actor"):
		print("select wire as actor")
		fst_selected_actor = null
		snd_selected_actor = null
		selected_actor_type = ActorType.Wire	
	elif event.is_action_pressed("select_actor"):
		if (fst_selected_actor == null):
			fst_selected_actor = get_looking_at_actor()
			print("selected first actor")
		else:
			snd_selected_actor = get_looking_at_actor()
			print("selected second actor")
	elif event is InputEventMouseMotion:
		accumulated_mouse += event.relative * sens
		
var machine_scene = preload("res://machine.tscn")
var power_pole_scene = preload("res://pole.tscn")
var wire_scene = preload("res://wire.tscn")


		
func place_actor() -> void:
	var instance = null
	
	match selected_actor_type:
		ActorType.Machine:
			instance = machine_scene.instantiate()
		ActorType.PowerPole:
			instance = power_pole_scene.instantiate()
		ActorType.Wire:
			if (fst_selected_actor == null || snd_selected_actor == null):
				print("need to select two actors to wire up")
				return
			instance = wire_scene.instantiate()
	
	var node = instance as Node3D
	node.position += raycaster.get_collision_point() + Vector3(0, 0.5, 0)
	
	match selected_actor_type:
		ActorType.Wire:
			var node_p1 = (fst_selected_actor as Node3D).position
			var node_p2 = (snd_selected_actor as Node3D).position
			var d = node_p1.distance_to(node_p2)
			var pos = (node_p1 + node_p2) * 0.5
			node.position = pos
			node.look_at_from_position(pos, node_p1)
			
			var col_shape = node.get_node("StaticBody3D/CollisionShape3D") as CollisionShape3D
			col_shape.shape = col_shape.shape.duplicate()
			(col_shape.shape as BoxShape3D).size.z = d
			
			var mesh = node.get_node("MeshInstance3D") as Node3D 
			mesh.scale.y = d * 0.5
			
			var wire = node as WireNode
			wire.pole_1_ref = fst_selected_actor as PoleNode
			wire.pole_2_ref = snd_selected_actor as PoleNode
			
			
			fst_selected_actor = null
			snd_selected_actor = null
		_:
			pass
	
	get_tree().root.get_child(0).add_child(instance)
	
func remove_actor() -> void: 
	var collider = raycaster.get_collider()
	
	if (collider == null):
		return
	
	var parent = collider.get_parent();
	if (parent.is_in_group("actors")):
		parent.queue_free()
