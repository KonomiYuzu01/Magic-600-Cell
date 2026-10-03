cmake_minimum_required(VERSION 3.24)
project(m600_sb_probe LANGUAGES CXX)
if(NOT WIN32 OR NOT MSVC)
    message(FATAL_ERROR "This probe builds with Windows SDK and MSVC x64")
endif()
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
get_filename_component(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../.." ABSOLUTE)
find_program(DXC_EXECUTABLE NAMES dxc REQUIRED)
add_executable(sb_probe src/probe.cpp src/gpu.cpp src/edges.cpp)
target_compile_definitions(sb_probe PRIVATE WIN32_LEAN_AND_MEAN NOMINMAX UNICODE _UNICODE
    M600_REPO_ROOT="${REPO_ROOT}")
target_compile_options(sb_probe PRIVATE /W4 /permissive- /utf-8 /EHsc)
target_link_libraries(sb_probe PRIVATE d3d12 dxgi dxguid bcrypt user32 dwmapi powrprof windowscodecs ole32)
function(compile_shader output source entry profile)
    add_custom_command(OUTPUT "${CMAKE_CURRENT_BINARY_DIR}/${output}"
        COMMAND "${DXC_EXECUTABLE}" -nologo -O3 -Ges -T "${profile}" -E "${entry}"
            ${ARGN} -I "${CMAKE_CURRENT_SOURCE_DIR}/shaders"
            -Fo "${CMAKE_CURRENT_BINARY_DIR}/${output}"
            "${CMAKE_CURRENT_SOURCE_DIR}/shaders/${source}"
        DEPENDS "shaders/${source}" "shaders/geometry.hlsl" VERBATIM)
endfunction()
compile_shader(draw_vs.dxil draw.hlsl vsMain vs_6_0)
compile_shader(draw_ps.dxil draw.hlsl psMain ps_6_0)
compile_shader(count_vs.dxil draw.hlsl vsMain vs_6_0 -DCOUNT_INVOCATIONS=1)
compile_shader(geometry_cs.dxil check.hlsl csMain cs_6_0)
compile_shader(fog_ps.dxil fog.hlsl fogMain ps_6_0)
compile_shader(outline_vs.dxil outlines.hlsl outlineVS vs_6_0)
compile_shader(outline_ps.dxil outlines.hlsl outlinePS ps_6_0)
compile_shader(transparency_vs.dxil transparency.hlsl transparencyVS vs_6_0)
compile_shader(transparency_ps.dxil transparency.hlsl transparencyPS ps_6_0)
compile_shader(sort_keys_cs.dxil sort.hlsl keyMain cs_6_0)
compile_shader(sort_cs.dxil sort.hlsl sortMain cs_6_0)
compile_shader(sort_prefix_cs.dxil sort.hlsl prefixMain cs_6_0)
compile_shader(sort_blocks_cs.dxil sort.hlsl blocksMain cs_6_0)
compile_shader(sort_indices_cs.dxil sort.hlsl indicesMain cs_6_0)
compile_shader(post_vs.dxil post.hlsl fullscreenVS vs_6_0)
compile_shader(dof_ps.dxil post.hlsl dofPS ps_6_0)
compile_shader(ao_ps.dxil post.hlsl aoPS ps_6_0)
compile_shader(ao_blur_ps.dxil post.hlsl aoBlurPS ps_6_0)
compile_shader(ao_composite_ps.dxil post.hlsl aoCompositePS ps_6_0)
add_custom_target(sb_shaders ALL DEPENDS
    "${CMAKE_CURRENT_BINARY_DIR}/draw_vs.dxil" "${CMAKE_CURRENT_BINARY_DIR}/draw_ps.dxil"
    "${CMAKE_CURRENT_BINARY_DIR}/count_vs.dxil" "${CMAKE_CURRENT_BINARY_DIR}/geometry_cs.dxil"
    "${CMAKE_CURRENT_BINARY_DIR}/fog_ps.dxil"
    "${CMAKE_CURRENT_BINARY_DIR}/outline_vs.dxil" "${CMAKE_CURRENT_BINARY_DIR}/outline_ps.dxil"
    "${CMAKE_CURRENT_BINARY_DIR}/transparency_vs.dxil" "${CMAKE_CURRENT_BINARY_DIR}/transparency_ps.dxil"
    "${CMAKE_CURRENT_BINARY_DIR}/sort_keys_cs.dxil" "${CMAKE_CURRENT_BINARY_DIR}/sort_cs.dxil"
    "${CMAKE_CURRENT_BINARY_DIR}/sort_prefix_cs.dxil" "${CMAKE_CURRENT_BINARY_DIR}/sort_blocks_cs.dxil"
    "${CMAKE_CURRENT_BINARY_DIR}/sort_indices_cs.dxil"
    "${CMAKE_CURRENT_BINARY_DIR}/post_vs.dxil" "${CMAKE_CURRENT_BINARY_DIR}/dof_ps.dxil"
    "${CMAKE_CURRENT_BINARY_DIR}/ao_ps.dxil" "${CMAKE_CURRENT_BINARY_DIR}/ao_blur_ps.dxil"
    "${CMAKE_CURRENT_BINARY_DIR}/ao_composite_ps.dxil")
add_dependencies(sb_probe sb_shaders)
