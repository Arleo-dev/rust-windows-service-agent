cmake_minimum_required(VERSION 3.14)
project(LoggerChild VERSION 1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_executable(logger_child src/main.cpp)