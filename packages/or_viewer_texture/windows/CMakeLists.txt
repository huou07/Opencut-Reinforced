cmake_minimum_required(VERSION 3.14)
project(or_viewer_texture LANGUAGES CXX)

add_library(or_viewer_texture_plugin SHARED
  "or_viewer_texture_plugin.cpp"
  "or_viewer_texture_plugin_c_api.cpp"
)
apply_standard_settings(or_viewer_texture_plugin)
target_include_directories(or_viewer_texture_plugin PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/include")
target_include_directories(or_viewer_texture_plugin PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/../native")
target_link_libraries(or_viewer_texture_plugin PRIVATE flutter flutter_wrapper_plugin)
set(or_viewer_texture_bundled_libraries "" PARENT_SCOPE)
