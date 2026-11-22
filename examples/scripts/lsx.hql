#!/usr/bin/env hql
// status: valid-now
// implementation: implemented
// environment: pure
// feature: A script lists each folder named on its command line, joined to the working directory
// expected-type: Step
// expected: ()

from io.fs import *
import io.env

// The names in each folder given, or in the directory the script was run
// from. A folder on the command line is named from where the user stands,
// so it is joined to the working directory, not resolved beside the script.
here = working_dir()
folders = env.argv() | map(p => folder(here.join(p))) given env.argv() != []
          otherwise [folder(here)]

folders | flat_map(f => entries(f)) | map(e => e.name) | sink(stdout, Text)
