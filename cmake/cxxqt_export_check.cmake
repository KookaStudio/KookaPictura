# Touch STAMP when OBJECT is missing.
#
# The cxx-qt initializer object is written by the crate's build script, but the
# C++ build declares it as the output of a no-op step, so a generator clean
# deletes it while cargo still considers the build script fresh. The build
# script watches STAMP, so touching it makes the next cargo step write the
# object again. When the object exists the stamp is left alone, so cargo does
# not rerun the build script on every build.
if(NOT EXISTS "${OBJECT}")
    file(TOUCH "${STAMP}")
endif()
