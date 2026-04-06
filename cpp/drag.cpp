#include "drag.h"

#ifdef _WIN32

#include <algorithm>
#include <windows.h>

namespace {

HWND handle_to_hwnd(long long handle) {
    return reinterpret_cast<HWND>(static_cast<intptr_t>(handle));
}

bool get_window_rect(HWND hwnd, RECT* rect) {
    return rect != nullptr && GetWindowRect(hwnd, rect) != 0;
}

bool get_monitor_info_for_window(HWND hwnd, MONITORINFO* info) {
    if (info == nullptr) {
        return false;
    }

    info->cbSize = sizeof(MONITORINFO);
    HMONITOR monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    return GetMonitorInfoW(monitor, info) != 0;
}

bool get_monitor_info_for_point(int x, int y, MONITORINFO* info) {
    if (info == nullptr) {
        return false;
    }

    info->cbSize = sizeof(MONITORINFO);
    POINT point { x, y };
    HMONITOR monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
    return GetMonitorInfoW(monitor, info) != 0;
}

bool is_fullscreen_or_unsupported(HWND hwnd) {
    RECT rect {};
    MONITORINFO monitor_info {};
    if (!get_window_rect(hwnd, &rect) || !get_monitor_info_for_window(hwnd, &monitor_info)) {
        return true;
    }

    const bool covers_monitor =
        rect.left <= monitor_info.rcMonitor.left &&
        rect.top <= monitor_info.rcMonitor.top &&
        rect.right >= monitor_info.rcMonitor.right &&
        rect.bottom >= monitor_info.rcMonitor.bottom;

    const auto style = static_cast<unsigned long>(GetWindowLongW(hwnd, GWL_STYLE));
    const auto ex_style = static_cast<unsigned long>(GetWindowLongW(hwnd, GWL_EXSTYLE));
    const bool has_caption = (style & WS_CAPTION) != 0;
    const bool topmost = (ex_style & WS_EX_TOPMOST) != 0;

    return covers_monitor && (!has_caption || topmost);
}

void restore_maximized_window(HWND hwnd, int cursor_x, int cursor_y) {
    ShowWindow(hwnd, SW_RESTORE);
    Sleep(10);

    RECT rect {};
    MONITORINFO monitor_info {};
    if (!get_window_rect(hwnd, &rect) || !get_monitor_info_for_point(cursor_x, cursor_y, &monitor_info)) {
        return;
    }

    const int width = rect.right - rect.left;
    const int height = rect.bottom - rect.top;
    const int grip_offset = std::min(24, std::max(8, height / 3));
    const int min_x = monitor_info.rcWork.left;
    const int max_x = monitor_info.rcWork.right - width;
    const int min_y = monitor_info.rcWork.top;
    const int max_y = monitor_info.rcWork.bottom - height;
    const int target_x = std::max(min_x, std::min(max_x, cursor_x - (width / 2)));
    const int target_y = std::max(min_y, std::min(max_y, cursor_y - grip_offset));

    SetWindowPos(
        hwnd,
        nullptr,
        target_x,
        target_y,
        0,
        0,
        SWP_ASYNCWINDOWPOS | SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER
    );

    Sleep(5);
}

}  // namespace

DRAG_API int drag_bootstrap_process() {
    return SetProcessDPIAware() != 0;
}

DRAG_API int drag_prepare_foreground_window(int cursor_x, int cursor_y, DragWindowState* out_state) {
    if (out_state == nullptr) {
        return 0;
    }

    HWND hwnd = GetForegroundWindow();
    if (hwnd == nullptr || IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0) {
        return 0;
    }

    if (is_fullscreen_or_unsupported(hwnd)) {
        return 0;
    }

    const int was_maximized = IsZoomed(hwnd) != 0;
    if (was_maximized) {
        restore_maximized_window(hwnd, cursor_x, cursor_y);
    }

    RECT rect {};
    if (!get_window_rect(hwnd, &rect)) {
        return 0;
    }

    out_state->handle = static_cast<long long>(reinterpret_cast<intptr_t>(hwnd));
    out_state->x = rect.left;
    out_state->y = rect.top;
    out_state->width = rect.right - rect.left;
    out_state->height = rect.bottom - rect.top;
    out_state->was_maximized = was_maximized;
    return 1;
}

DRAG_API int drag_move_window(long long handle, int x, int y) {
    HWND hwnd = handle_to_hwnd(handle);
    if (hwnd == nullptr) {
        return 0;
    }

    return SetWindowPos(
        hwnd,
        nullptr,
        x,
        y,
        0,
        0,
        SWP_ASYNCWINDOWPOS | SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER
    ) != 0;
}

DRAG_API int drag_window_is_valid(long long handle) {
    HWND hwnd = handle_to_hwnd(handle);
    return hwnd != nullptr && IsWindow(hwnd) != 0;
}

DRAG_API int drag_get_cursor_position(int* x, int* y) {
    if (x == nullptr || y == nullptr) {
        return 0;
    }

    POINT point {};
    if (GetCursorPos(&point) == 0) {
        return 0;
    }

    *x = point.x;
    *y = point.y;
    return 1;
}

#else

DRAG_API int drag_bootstrap_process() { return 0; }
DRAG_API int drag_prepare_foreground_window(int, int, DragWindowState*) { return 0; }
DRAG_API int drag_move_window(long long, int, int) { return 0; }
DRAG_API int drag_window_is_valid(long long) { return 0; }
DRAG_API int drag_get_cursor_position(int*, int*) { return 0; }

#endif
