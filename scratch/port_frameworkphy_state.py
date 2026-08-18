import re

with open("scratch/frameworkphy.cc.old") as f:
    cc = f.read()

with open("src/libemane/frameworkphy.h") as f:
    h = f.read()

print("Parsed.")
