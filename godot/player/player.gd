extends CharacterBody3D

@export var sens = 0.001
@export var speed = 5.0
@export var jump_velocity = 4.5
@export var push_force = 0.2

@onready var camera = $"Camera3D"
@onready var raycaster = $"Camera3D/RayCast3D"
@onready var indicator = $"Select Indicator"


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
	accumulated_mouse.y = clamp(accumulated_mouse.y, -PI * 0.5, PI * 0.5)
	var x = -accumulated_mouse.y
	var y = -accumulated_mouse.x
	
	camera.rotation = Vector3(x, 0, 0)
	
	# TODO: there's still jittery rotation happening here, probably because of player rotation interpolation
	rotation = Vector3(0, y, 0)
	
	
	# moves the select indicator to where the raycast hit (if it did hit)
	if (raycaster.get_collider() == null):
		indicator.hide()
	else:
		indicator.show()
		indicator.global_position = raycaster.get_collision_point()
	
	pass

enum ActorType {
	Machine,
	PowerPole,
	Silo,
	Wire,
	Belt
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

var mouse_visible: bool = false
	
func _input(event):
	if event.is_action_pressed("ui_cancel"):
		mouse_visible = !mouse_visible
		
		if (mouse_visible):
			Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
		else:
			Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
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
	elif event.is_action_pressed("select_silo_as_actor"):
		print("select silo as actor")
		selected_actor_type = ActorType.Silo
	elif event.is_action_pressed("select_wire_as_actor"):
		print("select wire as actor")
		fst_selected_actor = null
		snd_selected_actor = null
		selected_actor_type = ActorType.Wire	
	elif event.is_action_pressed("select_belt_as_actor"):
		print("select belt as actor")
		fst_selected_actor = null
		snd_selected_actor = null
		selected_actor_type = ActorType.Belt	
	elif event.is_action_pressed("select_actor"):
		if (fst_selected_actor == null):
			fst_selected_actor = get_looking_at_actor()
			print("selected first actor")
		else:
			snd_selected_actor = get_looking_at_actor()
			print("selected second actor")
	elif event is InputEventMouseMotion:
		accumulated_mouse += event.relative * sens
		
var machine_scene = preload("res://actors/machine.tscn")
var power_pole_scene = preload("res://actors/pole.tscn")
var silo_scene = preload("res://actors/silo.tscn")
var wire_scene = preload("res://actors/wire.tscn")
var belt_scene = preload("res://actors/belt.tscn")


		
func place_actor() -> void:
	var instance = null
	
	match selected_actor_type:
		ActorType.Machine:
			instance = machine_scene.instantiate()
		ActorType.PowerPole:
			instance = power_pole_scene.instantiate()
		ActorType.Silo:
			instance = silo_scene.instantiate()
		ActorType.Wire:
			if ((fst_selected_actor as PoleNode) == null || (snd_selected_actor as PoleNode) == null):
				print("need to select two poles to wire up")
				return
			instance = wire_scene.instantiate()
		ActorType.Belt:
			if ((fst_selected_actor as HatchNode) == null || (snd_selected_actor as HatchNode) == null):
				print("need to select two hatches to belt up")
				return
			instance = belt_scene.instantiate()
	
	var node = instance as Node3D
	var position = Vector3.ZERO
	
	match selected_actor_type:
		ActorType.Wire:
			var node_p1 = (fst_selected_actor.get_node("Connector") as Node3D).global_position
			var node_p2 = (snd_selected_actor.get_node("Connector") as Node3D).global_position
			var d = node_p1.distance_to(node_p2)
			var pos = (node_p1 + node_p2) * 0.5
			position = pos
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
		ActorType.Belt:
			var node_p1 = (fst_selected_actor as Node3D).global_position
			var node_p2 = (snd_selected_actor as Node3D).global_position
			var d = node_p1.distance_to(node_p2)
			var pos = (node_p1 + node_p2) * 0.5
			position = pos
			node.look_at_from_position(pos, node_p1)
			
			var col_shape = node.get_node("StaticBody3D/CollisionShape3D") as CollisionShape3D
			col_shape.shape = col_shape.shape.duplicate()
			(col_shape.shape as BoxShape3D).size.z = d
			
			var mesh = node.get_node("Belt Visual") as Node3D 
			mesh.scale.z = d * 0.5
			
			var belt = node as BeltNode
			belt.belt_start_hatch_ref = fst_selected_actor as HatchNode
			belt.belt_end_hatch_ref = snd_selected_actor as HatchNode
			
			
			fst_selected_actor = null
			snd_selected_actor = null
		_:
			position = raycaster.get_collision_point() + Vector3(0, 0.5, 0)
	
	get_tree().root.get_child(0).add_child(instance)
	node.global_position = position
	
	
func remove_actor() -> void: 
	var collider = raycaster.get_collider()
	
	if (collider == null):
		return
	
	var parent = collider.get_parent();
	if (parent.is_in_group("actors")):
		# check if actor is hatch (cannot destroy)
		if (parent is HatchNode):
			return
		
		# check if actor is pole and is owned by machine (and, in which case, we cannot destroy it)
		var pole = parent as PoleNode
		if (pole != null):
			if (!pole.owned):
				parent.queue_free()
		else:
			parent.queue_free()
