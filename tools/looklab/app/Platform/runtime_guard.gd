extends Node

func _enter_tree() -> void:
    if not preload("res://Platform/directory_guard.gd").report():
        get_tree().quit(1)
