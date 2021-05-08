from cleanax import clean

files_to_delete = clean("/run/media/vincent/WaifuChan DataSet/good-images/images")

for f in files_to_delete:
    print(f)