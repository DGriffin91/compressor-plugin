import matplotlib.pyplot as plt
from math import *


def gain_from_db(decibels):
    return pow(10.0, decibels * 0.05)


def db_from_gain(gain):
    return log(gain, 10.0) * 20.0


def mix(x, y, a):
    return x * (1.0 - a) + y * a


def to_range(bottom, top, x):
    return x * (top - bottom) + bottom


def from_range(bottom, top, x):
    return (x - bottom) / (top - bottom)


def clamp(x, min_v, max_v):
    return max(min(x, max_v), min_v)


def smoothstep(edge0, edge1, x):
    # Scale, bias and saturate x to 0..1 range
    x = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0)
    # Evaluate polynomial
    return x * x * (3.0 - 2.0 * x)


threshold = gain_from_db(-6.0)
knee = gain_from_db(3.0)

attack = 10
release = 100
sample_rate = 48000
ratio = 2

slope = 1.0 / ratio - 1.0

attack_gain = exp(-2.0 * pi * 1000.0 / attack / sample_rate)
release_gain = exp(-2.0 * pi * 1000.0 / release / sample_rate)

env = 0.0
cv_env = 1.0
data = []
data2 = []


def cubic(p0, p1, p2, p3, t):
    return (
        (1 - t) ** 3 * p0
        + 3 * t * (1 - t) ** 2 * p1
        + 3 * t ** 2 * (1 - t) * p2
        + t ** 3 * p3
    )


def quadratic(p0, p1, p2, t):
    return (1 - t) ** 2 * p0 + 2 * (1 - t) * t * p1 + t * t * p2


def reiss(db, th_db, width, ratio):
    if 2 * (db - th_db) < -width:
        return db
    elif 2 * abs(db - th_db) <= width:
        return db + (1 / ratio - 1) * (db - th_db + width / 2) ** 2 / (2 * width)
    elif 2 * (db - th_db) > width:
        return th_db + (db - th_db) / ratio


def basic(env, threshold, ratio):
    if detector_input > threshold:
        return pow(detector_input / threshold, slope)
    return 1.0


for i in range(1, 1000):
    detector_input = min((i / 1000), 1.0)
    data.append(detector_input)

    env = detector_input + attack_gain * (env - detector_input)
    env_db = db_from_gain(env)

    # cv = basic(env, threshold, ratio)
    cv = gain_from_db(
        reiss(env_db, db_from_gain(threshold), db_from_gain(knee), ratio) - env_db
    )

    data2.append(cv * detector_input)

plt.plot(data)
plt.plot(data2)

plt.show()

