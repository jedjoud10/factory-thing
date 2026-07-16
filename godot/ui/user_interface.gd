extends Control

@onready var player: CharacterBody3D = $"../Player"
@onready var top_label = $"PanelContainer/VBoxContainer/Label"
@onready var progress_bar = $"PanelContainer/VBoxContainer/ProgressBar"
@onready var selected_actor_label = $"Selected Actor Color Rect/Label2"

func _process(delta: float) -> void:	
	var raycaster: RayCast3D = player.raycaster
	top_label.hide()
	progress_bar.hide()
	if (raycaster.get_collider() != null):
		var obj: Node = raycaster.get_collider().get_parent();
		if (obj.has_method("get_ui_info")):
			top_label.show()
			top_label.text = obj.get_ui_info()
		if (obj.has_method("get_ui_progress_bar_percentage")):
			progress_bar.show()
			progress_bar.value = obj.get_ui_progress_bar_percentage()
			
	selected_actor_label.text = player.ActorType.keys()[player.selected_actor_type]
