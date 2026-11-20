cmake_minimum_required(VERSION 3.20)
project(sa2_core LANGUAGES CXX)
if(NOT WIN32 OR NOT MSVC)
    message(FATAL_ERROR "The SA2 renderer core requires MSVC and the Windows SDK")
endif()
set(CMAKE_MSVC_RUNTIME_LIBRARY MultiThreaded)
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}")
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY_RELEASE "${CMAKE_BINARY_DIR}")
# Every source is a product copy under renderer/core (see PROVENANCE.md). The
# repository root is used only at run time, for the model assets and the
# immutable references the readers check.
get_filename_component(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../.." ABSOLUTE)
set(CORE_SHADERS "${CMAKE_CURRENT_SOURCE_DIR}/shaders")
find_program(DXC_EXECUTABLE NAMES dxc REQUIRED)
# S-B's readers and checks (W1..W4) and W-J's (W-J scenes) are compiled as
# copied. Rename only their unused standalone entry points so the self-test
# links the same libraries without a second main. Both readers call the
# C++20-deprecated std::filesystem::u8path; silencing that diagnostic keeps the
# build /W4 clean without editing the copied sources.
add_library(sa2_sb STATIC src/sb/probe.cpp src/sb/edges.cpp src/sb_link.cpp)
target_include_directories(sa2_sb PUBLIC src/sb)
target_compile_definitions(sa2_sb PRIVATE M600_REPO_ROOT="${REPO_ROOT}" main=sa2_unused_sb_main _SILENCE_CXX20_U8PATH_DEPRECATION_WARNING)
target_link_libraries(sa2_sb PUBLIC bcrypt)
add_library(sa2_wj STATIC src/wj/probe.cpp src/wj_link.cpp)
target_include_directories(sa2_wj PRIVATE src src/sb)
target_compile_definitions(sa2_wj PRIVATE M600_REPO_ROOT="${REPO_ROOT}" main=sa2_unused_wj_main _SILENCE_CXX20_U8PATH_DEPRECATION_WARNING)
target_link_libraries(sa2_wj PUBLIC bcrypt)
add_library(sa2_pure STATIC src/pure.cpp)
target_include_directories(sa2_pure PUBLIC include src)
add_library(sa2_scene_record STATIC src/scene_record.cpp src/product.cpp src/stage_rules.cpp)
target_include_directories(sa2_scene_record PUBLIC include src)
target_link_libraries(sa2_scene_record PUBLIC sa2_sb sa2_wj)
add_library(sa2_interop SHARED src/interop.cpp src/scene.cpp src/stages.cpp)
target_compile_definitions(sa2_interop PRIVATE SA2_INTEROP_EXPORTS)
# ABI 5's route B (accepted for now by the owner on 18 Nov 2026, pending three
# owner-machine checks) adds the DirectComposition layer: dcomp, and user32 for
# IsWindow.
target_link_libraries(sa2_interop PRIVATE sa2_pure sa2_scene_record d3d12 dxgi dxguid dcomp user32)
# C9 U5: user shader stages. DXC runs at run time from dxcompiler.dll and dxil.dll beside the DLL (loaded by
# absolute path, never linked); ole32 frees the strings its version interfaces return. The core's own HLSL is
# embedded at build time (embed.cmake).
target_link_libraries(sa2_interop PRIVATE ole32)
add_executable(sa2_selftest src/selftest.cpp src/gpu_test.cpp)
target_link_libraries(sa2_selftest PRIVATE sa2_pure sa2_scene_record sa2_interop d3d12 dxgi dxguid)
foreach(target sa2_sb sa2_wj sa2_pure sa2_scene_record sa2_interop sa2_selftest)
    target_compile_features(${target} PRIVATE cxx_std_20)
    target_compile_definitions(${target} PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
    target_compile_options(${target} PRIVATE /W4 /permissive- /utf-8 /EHsc)
endforeach()
# W-J's pose-path shaders; W1..W4 run them with the pose path disabled. ABI 4's
# product passes (product.hlsl) include draw.hlsl and share its vertex stage.
function(compile_shader output source entry profile)
    add_custom_command(OUTPUT "${CMAKE_CURRENT_BINARY_DIR}/${output}"
        COMMAND "${DXC_EXECUTABLE}" -nologo -O3 -Ges -T "${profile}" -E "${entry}"
            ${ARGN} -I "${CORE_SHADERS}" -Fo "${CMAKE_CURRENT_BINARY_DIR}/${output}"
            "${CORE_SHADERS}/${source}"
        DEPENDS "${CORE_SHADERS}/${source}" "${CORE_SHADERS}/geometry.hlsl" "${CORE_SHADERS}/draw.hlsl" VERBATIM)
endfunction()
compile_shader(draw_vs.dxil draw.hlsl vsMain vs_6_0)
compile_shader(draw_ps.dxil draw.hlsl psMain ps_6_0)
compile_shader(count_vs.dxil draw.hlsl vsMain vs_6_0 -DCOUNT_INVOCATIONS=1)
compile_shader(geometry_cs.dxil check.hlsl csMain cs_6_0)
compile_shader(lattice_cs.dxil lattice.hlsl latticeMain cs_6_0)
compile_shader(id_ps.dxil product.hlsl idPs ps_6_0)
compile_shader(product_ps.dxil product.hlsl productPs ps_6_0)
compile_shader(product_vs.dxil product.hlsl productVs vs_6_0)
add_custom_target(sa2_shaders ALL DEPENDS "${CMAKE_CURRENT_BINARY_DIR}/draw_vs.dxil"
    "${CMAKE_CURRENT_BINARY_DIR}/draw_ps.dxil" "${CMAKE_CURRENT_BINARY_DIR}/count_vs.dxil"
    "${CMAKE_CURRENT_BINARY_DIR}/geometry_cs.dxil" "${CMAKE_CURRENT_BINARY_DIR}/lattice_cs.dxil"
    "${CMAKE_CURRENT_BINARY_DIR}/id_ps.dxil" "${CMAKE_CURRENT_BINARY_DIR}/product_ps.dxil"
    "${CMAKE_CURRENT_BINARY_DIR}/product_vs.dxil")
add_dependencies(sa2_interop sa2_shaders)
set(STAGE_SOURCES "${CMAKE_CURRENT_BINARY_DIR}/generated/stage_sources.h")
add_custom_command(OUTPUT "${STAGE_SOURCES}"
    COMMAND "${CMAKE_COMMAND}" -D "SHADERS=${CORE_SHADERS}" -D "OUTPUT=${STAGE_SOURCES}"
        -P "${CMAKE_CURRENT_SOURCE_DIR}/embed.cmake"
    DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/embed.cmake" "${CORE_SHADERS}/geometry.hlsl" "${CORE_SHADERS}/draw.hlsl"
        "${CORE_SHADERS}/stages.hlsl" VERBATIM)
add_custom_target(sa2_stage_sources ALL DEPENDS "${STAGE_SOURCES}")
add_dependencies(sa2_interop sa2_stage_sources)
target_include_directories(sa2_interop PRIVATE "${CMAKE_CURRENT_BINARY_DIR}/generated")
# The self-test's --stages run recomputes the composed source's digest from the same embedded text.
add_dependencies(sa2_selftest sa2_stage_sources)
target_include_directories(sa2_selftest PRIVATE "${CMAKE_CURRENT_BINARY_DIR}/generated")
