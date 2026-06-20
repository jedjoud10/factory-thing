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

func _input(event):
	if event.is_action_pressed("ui_cancel"):
		Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
	elif event.is_action_pressed("place_machine"):
		place_machine()
	elif event is InputEventMouseMotion:
		accumulated_mouse += event.relative * sens
		
var scene = preload("res://machine.tscn")
		
func place_machine() -> void:
	var instance = scene.instantiate()
	var node = instance as Node3D
	node.position = raycaster.get_collision_point()
	get_tree().root.add_child(instance)
