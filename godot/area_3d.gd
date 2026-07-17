extends Area3D

var attached_rb: RigidBody3D = null

func _on_body_entered(body: Node3D) -> void:
	if (attached_rb != null):
		return
	
	if (body is RigidBody3D):
		(body as RigidBody3D).freeze = true
		var tween = get_tree().create_tween()
		tween.tween_property(body, "global_position", global_position, 0.2)
		tween.tween_property(body, "global_basis", Basis.IDENTITY, 0.2)
		attached_rb = (body as RigidBody3D)


func _on_body_exited(body: Node3D) -> void:
	if (body is RigidBody3D):
		if ((body as RigidBody3D) == attached_rb):
			attached_rb = null
	pass # Replace with function body.
