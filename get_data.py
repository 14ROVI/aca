import subprocess

scripts = ["inner_product", "gcd", "fibonacci", "merge_sort", "matmul", "box_blur"]

data = {}

for script in scripts:
    fp = f".\scripts\{script}.acasm"
    print(f"NOW EXECUTING {fp}")
    data[script] = {}
    for i in range(1, 64, 2):
        out = subprocess.check_output(["cargo", "run", "--release", "--", fp, "--rob-size", str(i)])
        rate = float(out.decode("utf-8").split("Comitted Ops/Cycle: ")[-1].splitlines()[0])
        data[script][i] = rate
        
print(data)