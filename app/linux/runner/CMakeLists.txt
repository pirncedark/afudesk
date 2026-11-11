cmake_minimum_required(VERSION 3.13)
add_executable(${BINARY_NAME}
  "main.cc"
  "my_application.cc"
  "google_secure_storage.cc"
  "${FLUTTER_MANAGED_DIR}/generated_plugin_registrant.cc"
)
apply_standard_settings(${BINARY_NAME})
target_compile_definitions(${BINARY_NAME} PRIVATE APPLICATION_ID="${APPLICATION_ID}")
target_link_libraries(${BINARY_NAME} PRIVATE flutter PkgConfig::GTK PkgConfig::SECRET OpenSSL::Crypto)
target_include_directories(${BINARY_NAME} PRIVATE "${CMAKE_SOURCE_DIR}")
