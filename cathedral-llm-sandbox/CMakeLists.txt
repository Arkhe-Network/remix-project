cmake_minimum_required(VERSION 3.10)
project(cathedral_llm)

set(CMAKE_C_STANDARD 11)

add_executable(cathedral_llm_sandbox cathedral_llm_sandbox.c)
target_link_libraries(cathedral_llm_sandbox -lseccomp)

add_library(cathedral_ipc cathedral_ipc.c)
target_link_libraries(cathedral_ipc)
