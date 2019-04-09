cmake_minimum_required(VERSION 3.7.2)

add_executable(root_task IMPORTED)
set_property(TARGET root_task PROPERTY IMPORTED_LOCATION  "$ENV{ROOT_TASK_PATH}")

include($ENV{SEL4_TOOLS_DIR}/cmake-tool/helpers/rootserver.cmake)
include($ENV{SEL4_TOOLS_DIR}/cmake-tool/base.cmake)
include($ENV{SEL4_TOOLS_DIR}/cmake-tool/configuration.cmake)

DeclareRootserver(root_task)
