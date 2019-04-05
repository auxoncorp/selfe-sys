cmake_minimum_required(VERSION 3.7.2)

# base.cmake requires this be present
add_custom_target(rootserver_image)

include(/home/mullr/devel/libsel4-sys/deps/seL4_tools/cmake-tool/base.cmake)
include(/home/mullr/devel/libsel4-sys/deps/seL4_tools/cmake-tool/configuration.cmake)
