extends Node3D

@onready var camera: Camera3D = $"../Camera3D"
@onready var raycaster: RayCast3D = $"../Camera3D/RayCast3D"
@onready var manager: FactoryManager = get_tree().root.get_child(0).find_child("FactoryManager")

var highlighted_actor: Node3D = null
var machine_scene = preload("res://actors/machine.tscn")
var power_pole_scene = preload("res://actors/pole.tscn")
var silo_scene = preload("res://actors/silo.tscn")
var wire_scene = preload("res://actors/wire.tscn")
var belt_scene = preload("res://actors/belt.tscn")

func _process(delta: float) -> void:
	# returns null if raycast misses
	var possible_actor = null	
	if (raycaster.get_collider() != null):
		possible_actor = raycaster.get_collider().get_parent()
	
	# invalidate possible actor if it is not an actor
	if (possible_actor != null && not possible_actor.is_in_group("actors")):
		possible_actor = null
	
	if (possible_actor == null):
		# disable highlight of previous actor when ray cast misses
		if (highlighted_actor != null):
			highlighted_actor.propagate_call("set", ["material_overlay", null])
			highlighted_actor = null
	else:
		if (highlighted_actor == null):
			highlighted_actor = possible_actor;
		elif highlighted_actor != possible_actor:
			highlighted_actor.propagate_call("set", ["material_overlay", null])
			highlighted_actor = possible_actor;
		highlighted_actor.propagate_call("set", ["material_overlay", preload("res://materials/highlight.tres")])
	
	# holograms are only visible when we have a target point where we can place actor
	if (raycaster.get_collider() != null):
		if (hologram_actor_type != selected_actor_type or hologram_actor == null):
			recreate_hologram_actor()
		
		
	update_hologram_if_any()
		
enum ActorType {
	Machine,
	PowerPole,
	Silo,
	Wire,
	Belt
}

func update_hologram_if_any() -> void:
	if (hologram_actor != null):
		hologram_actor.hide()
		
		# only update global position when we are not a wire or belt (because those are automatically handled)
		match selected_actor_type:
			ActorType.Wire:					
				if (fst_selected_actor != null):
					hologram_actor.show()
					var node_p1 = (fst_selected_actor.get_node("Connector") as Node3D).global_position
					
					var node_p2 = -camera.global_basis.z + camera.global_position
					if (raycaster.get_collider() != null):
						var parent = raycaster.get_collider().get_parent()		
						if (parent is PoleNode and parent != fst_selected_actor):
							node_p2 = (parent.get_node("Connector") as Node3D).global_position
					
					var d = node_p1.distance_to(node_p2)
					var pos = (node_p1 + node_p2) * 0.5
					hologram_actor.look_at_from_position(pos, node_p1)
					var mesh = hologram_actor.get_node("MeshInstance3D") as Node3D 
					mesh.scale.y = d * 0.5
			_:
				hologram_actor.show()
				hologram_actor.global_position = get_actor_position_from_raycast()
				pass

func get_actor_position_from_raycast() -> Vector3:
	return round(raycaster.get_collision_point() + Vector3(0, 0.5, 0))

var selected_actor_type: ActorType = ActorType.Machine
var hologram_actor: Node3D = null
var hologram_actor_type: ActorType = ActorType.Machine
var fst_selected_actor: Node3D = null
var snd_selected_actor: Node3D = null

func get_looking_at_actor() -> Node3D:
	if (raycaster.get_collider() != null):
		var parent = raycaster.get_collider().get_parent()
		if (parent.is_in_group("actors")):
			return parent
	return null
	
func get_fst_snd_actor() -> Node3D:
	var actor = get_looking_at_actor()
	
	if (actor is HatchNode or actor is PoleNode):
		return actor
	else:
		return null

func _input(event):
	if event.is_action_pressed("place_actor"):
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
			fst_selected_actor = get_fst_snd_actor()
			
			if (fst_selected_actor != null):
				print("selected first actor")
		else:
			snd_selected_actor = get_fst_snd_actor()
			if (snd_selected_actor == fst_selected_actor):
				print("cannot select same actor twice. resetting")
				snd_selected_actor = null
				fst_selected_actor = null
			elif (snd_selected_actor != null):
				print("selected second actor")
				place_actor()

func recreate_hologram_actor() -> void:
	if (hologram_actor != null):
		hologram_actor.queue_free()
		hologram_actor = null
	
	hologram_actor_type = selected_actor_type
	match selected_actor_type:
		ActorType.Machine:
			hologram_actor = machine_scene.instantiate()
		ActorType.PowerPole:
			hologram_actor = power_pole_scene.instantiate()
		ActorType.Silo:
			hologram_actor = silo_scene.instantiate()
		ActorType.Wire:
			if ((fst_selected_actor as PoleNode) != null):
				hologram_actor = wire_scene.instantiate()
			else:
				return
		ActorType.Belt:
			return
			

	print("recreate hologram actor")
	hologram_actor.find_child("StaticBody3D").process_mode = Node.PROCESS_MODE_DISABLED
	hologram_actor.propagate_call("set", ["process_mode", Node.PROCESS_MODE_DISABLED])
	hologram_actor.propagate_call("set", ["collision_mask", 0])
	hologram_actor.propagate_call("set", ["material_override", preload("res://materials/hologram.tres")])
	
	if ("hologram" in hologram_actor):
		hologram_actor.hologram = true
	
	get_tree().root.get_child(0).add_child(hologram_actor)

		
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
			
			if (manager.are_poles_connected(fst_selected_actor as PoleNode, snd_selected_actor as PoleNode)):
				print("cannot place wire")
				return
				
			instance = wire_scene.instantiate()
		ActorType.Belt:
			if ((fst_selected_actor as HatchNode) == null || (snd_selected_actor as HatchNode) == null):
				print("need to select two hatches to belt up")
				return
				
			if (manager.are_hatches_connected(fst_selected_actor as HatchNode, snd_selected_actor as HatchNode)):
				print("cannot place belt")
				return
				
			if (manager.is_hatch_connected(fst_selected_actor as HatchNode)):
				print("cannot place belt")
				return
				
			if (manager.is_hatch_connected(snd_selected_actor as HatchNode)):
				print("cannot place belt")
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
			position = get_actor_position_from_raycast()
	
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
