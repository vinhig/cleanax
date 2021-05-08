import cv2
import os

for f in os.listdir("."):
    if f.endswith(".png"):
        img = cv2.imread(f, 0)
        if cv2.countNonZero(img) == 0:
            print(f)
    else:
        print(f)