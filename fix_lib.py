with open("rust/emane-core/src/lib.rs", "r") as f:
    lines = f.read().split("\n")

out = []
for line in lines:
    if line.startswith('        // not used yet'):
        out.append('        include!(concat!(env!("OUT_DIR"), "/emaneremotecontrolportapi.rs"));')
    else:
        out.append(line)

with open("rust/emane-core/src/lib.rs", "w") as f:
    f.write("\n".join(out))
