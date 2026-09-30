cmake_minimum_required(VERSION 3.13)
project(or_viewer_texture LANGUAGES CXX)

add_library(or_viewer_texture_plugin SHARED
  "or_viewer_texture_plugin.cc"
)
apply_standard_settings(or_viewer_texture_plugin)
target_include_directories(or_viewer_texture_plugin PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/include")
target_include_directories(or_viewer_texture_plugin PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/../native")
target_link_libraries(or_viewer_texture_plugin PRIVATE flutter ${CMAKE_DL_LIBS})
set(or_viewer_texture_bundled_libraries "" PARENT_SCOPE)
