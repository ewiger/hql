// The standard library's implicit import list. Not a module: it has no name,
// cannot be imported, and is the one file at a module root with a role. Each
// statement here is run at the head of every source unit, under the rules of
// any import (HQL-0060): `import m` binds the module's name, and
// `from m import *` its functions unqualified and its receivers.
// `prelude = false` under `[modules]` in .hql/config.toml withholds the whole
// list, `core` and `collections` with it.
//
// See doc/proposals/HQL-0024/parts/03-standard-library.hmd.
import core
from core import *
import collections
from collections import *
import string
from string import *
import bytes
from bytes import *
import seq
from seq import *
import math
from math import *
import option
from option import *
import result
from result import *
import graph
from graph import *
import format
from format import *
import io
from io import *
import io.transaction
from io.transaction import *
