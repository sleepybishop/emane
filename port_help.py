import re

with open('src/libemane/frameworkphy.h', 'r') as f:
    text = f.read()

# Just extract the methods and member variables
print("--- Methods ---")
methods = re.findall(r'(\w+[\s\*&]*\w+\([^\)]*\)[^;]*;)', text)
for m in methods:
    print(m.strip().replace('\n', ' '))

print("--- Members ---")
members = re.findall(r'(\w+(?:<[^>]+>)?[\s\*&]+(\w+)_;)', text)
for m in members:
    print(m)

