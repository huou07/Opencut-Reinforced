cmake_minimum_required(VERSION 3.22)
project(or_viewer_texture_jni LANGUAGES CXX)

add_library(or_viewer_texture_jni SHARED viewer_texture_jni.cpp)
target_compile_features(or_viewer_texture_jni PRIVATE cxx_std_17)
target_link_libraries(or_viewer_texture_jni PRIVATE ${CMAKE_DL_LIBS})
