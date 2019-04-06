cmake_minimum_required(VERSION 3.7.2)

# base.cmake requires this be present
add_custom_target(rootserver_image)

include($ENV{SEL4_TOOLS_DIR}/cmake-tool/base.cmake)
include($ENV{SEL4_TOOLS_DIR}/cmake-tool/configuration.cmake)
