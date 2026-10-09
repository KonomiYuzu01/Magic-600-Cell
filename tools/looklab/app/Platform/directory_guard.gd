@tool
extends RefCounted

static var reported := false

static func report() -> bool:
    if reported:
        return true
    var paths := {
        "user": OS.get_user_data_dir(),
        "config": OS.get_config_dir(),
        "data": OS.get_data_dir(),
        "cache": OS.get_cache_dir(),
    }
    for kind in paths:
        print("LOOKLAB_DIR ", kind, " ", JSON.stringify(paths[kind]))
    print("LOOKLAB_DIRS_END")
    # No app or plugin work starts before the harness validates all four paths.
    if OS.read_string_from_stdin().strip_edges() != "LOOKLAB_DIRECTORY_GUARD_OK":
        push_error("Look Lab must be started through check.py's staging harness")
        return false
    reported = true
    return true
