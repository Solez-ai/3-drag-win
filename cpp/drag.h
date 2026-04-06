#pragma once

#ifdef _WIN32
#define DRAG_API extern "C" __declspec(dllexport)
#else
#define DRAG_API extern "C"
#endif

struct DragWindowState {
    long long handle;
    int x;
    int y;
    int width;
    int height;
    int was_maximized;
};

DRAG_API int drag_bootstrap_process();
DRAG_API int drag_prepare_foreground_window(int cursor_x, int cursor_y, DragWindowState* out_state);
DRAG_API int drag_move_window(long long handle, int x, int y);
DRAG_API int drag_window_is_valid(long long handle);
DRAG_API int drag_get_cursor_position(int* x, int* y);
