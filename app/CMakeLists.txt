add_library(imgui STATIC
    ${imgui_SOURCE_DIR}/imgui.cpp
    ${imgui_SOURCE_DIR}/imgui_draw.cpp
    ${imgui_SOURCE_DIR}/imgui_tables.cpp
    ${imgui_SOURCE_DIR}/imgui_widgets.cpp
    ${imgui_SOURCE_DIR}/imgui_demo.cpp
    ${imgui_SOURCE_DIR}/backends/imgui_impl_win32.cpp
    ${imgui_SOURCE_DIR}/backends/imgui_impl_dx11.cpp)

target_include_directories(imgui PUBLIC
    ${imgui_SOURCE_DIR}
    ${imgui_SOURCE_DIR}/backends)

add_executable(whatbubbles WIN32
    main.cpp
    app_shell.cpp)

target_link_libraries(whatbubbles PRIVATE
    imgui
    whatbubbles_cxx
    whatbubbles_core
    d3d11
    dxgi
    d3dcompiler
    dwmapi)

set_target_properties(whatbubbles PROPERTIES
    VS_DEBUGGER_WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
    OUTPUT_NAME "WhatBubbles")
