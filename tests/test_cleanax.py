from cleanax import clean
import os

files_to_delete = clean(".")

for f in files_to_delete:
    print(f)
    # os.remove(f)