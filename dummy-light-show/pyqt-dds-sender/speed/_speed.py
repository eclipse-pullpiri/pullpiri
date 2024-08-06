"""
  Generated by Eclipse Cyclone DDS idlc Python Backend
  Cyclone DDS IDL version: v0.10.5
  Module: speed
  IDL file: speed.idl

"""

from enum import auto
from typing import TYPE_CHECKING, Optional
from dataclasses import dataclass

import cyclonedds.idl as idl
import cyclonedds.idl.annotations as annotate
import cyclonedds.idl.types as types

# root module import for resolving types
import speed


@dataclass
@annotate.final
@annotate.autoid("sequential")
class DataType(idl.IdlStruct, typename="speed.DataType"):
    value: types.int32


