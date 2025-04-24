#----------------------------------------------------------------
# Generated CMake target import file for configuration "Debug".
#----------------------------------------------------------------

# Commands may need to know the format version.
set(CMAKE_IMPORT_FILE_VERSION 1)

# Import target "fltk::z" for configuration "Debug"
set_property(TARGET fltk::z APPEND PROPERTY IMPORTED_CONFIGURATIONS DEBUG)
set_target_properties(fltk::z PROPERTIES
  IMPORTED_LINK_INTERFACE_LANGUAGES_DEBUG "C"
  IMPORTED_LOCATION_DEBUG "${_IMPORT_PREFIX}/lib/libfltk_z.a"
  )

list(APPEND _cmake_import_check_targets fltk::z )
list(APPEND _cmake_import_check_files_for_fltk::z "${_IMPORT_PREFIX}/lib/libfltk_z.a" )

# Import target "fltk::png" for configuration "Debug"
set_property(TARGET fltk::png APPEND PROPERTY IMPORTED_CONFIGURATIONS DEBUG)
set_target_properties(fltk::png PROPERTIES
  IMPORTED_LINK_INTERFACE_LANGUAGES_DEBUG "C"
  IMPORTED_LOCATION_DEBUG "${_IMPORT_PREFIX}/lib/libfltk_png.a"
  )

list(APPEND _cmake_import_check_targets fltk::png )
list(APPEND _cmake_import_check_files_for_fltk::png "${_IMPORT_PREFIX}/lib/libfltk_png.a" )

# Import target "fltk::jpeg" for configuration "Debug"
set_property(TARGET fltk::jpeg APPEND PROPERTY IMPORTED_CONFIGURATIONS DEBUG)
set_target_properties(fltk::jpeg PROPERTIES
  IMPORTED_LINK_INTERFACE_LANGUAGES_DEBUG "C"
  IMPORTED_LOCATION_DEBUG "${_IMPORT_PREFIX}/lib/libfltk_jpeg.a"
  )

list(APPEND _cmake_import_check_targets fltk::jpeg )
list(APPEND _cmake_import_check_files_for_fltk::jpeg "${_IMPORT_PREFIX}/lib/libfltk_jpeg.a" )

# Import target "fltk::fltk" for configuration "Debug"
set_property(TARGET fltk::fltk APPEND PROPERTY IMPORTED_CONFIGURATIONS DEBUG)
set_target_properties(fltk::fltk PROPERTIES
  IMPORTED_LINK_INTERFACE_LANGUAGES_DEBUG "C;CXX"
  IMPORTED_LOCATION_DEBUG "${_IMPORT_PREFIX}/lib/libfltk.a"
  )

list(APPEND _cmake_import_check_targets fltk::fltk )
list(APPEND _cmake_import_check_files_for_fltk::fltk "${_IMPORT_PREFIX}/lib/libfltk.a" )

# Import target "fltk::forms" for configuration "Debug"
set_property(TARGET fltk::forms APPEND PROPERTY IMPORTED_CONFIGURATIONS DEBUG)
set_target_properties(fltk::forms PROPERTIES
  IMPORTED_LINK_INTERFACE_LANGUAGES_DEBUG "CXX"
  IMPORTED_LOCATION_DEBUG "${_IMPORT_PREFIX}/lib/libfltk_forms.a"
  )

list(APPEND _cmake_import_check_targets fltk::forms )
list(APPEND _cmake_import_check_files_for_fltk::forms "${_IMPORT_PREFIX}/lib/libfltk_forms.a" )

# Import target "fltk::images" for configuration "Debug"
set_property(TARGET fltk::images APPEND PROPERTY IMPORTED_CONFIGURATIONS DEBUG)
set_target_properties(fltk::images PROPERTIES
  IMPORTED_LINK_INTERFACE_LANGUAGES_DEBUG "CXX"
  IMPORTED_LOCATION_DEBUG "${_IMPORT_PREFIX}/lib/libfltk_images.a"
  )

list(APPEND _cmake_import_check_targets fltk::images )
list(APPEND _cmake_import_check_files_for_fltk::images "${_IMPORT_PREFIX}/lib/libfltk_images.a" )

# Commands beyond this point should not need to know the version.
set(CMAKE_IMPORT_FILE_VERSION)
