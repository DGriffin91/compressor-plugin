import matplotlib.pyplot as plt
from math import *


def lin_from_db(decibels):
    return pow(10.0, decibels * 0.05)


def db_from_lin(gain):
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


threshold = lin_from_db(-6.0)
knee = lin_from_db(6.0)

attack = 10
release = 50
sample_rate = 48000
ratio = 5

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


class DecoupledPeakDetector(object):
    def __init__(self, attack, release, sample_rate):
        super().__init__()
        self.attack_input = attack
        self.release_input = release
        self.env = 1
        self.env2 = 1
        self.sample_rate = sample_rate
        self.update()

    def process(self, x):
        self.env = max(x, self.release * self.env)
        self.env2 = self.attack * self.env2 + (1 - self.attack) * self.env
        return self.env2

    def process_smooth(self, x):
        self.env = max(x, self.release * self.env + (1 - self.release) * x)
        self.env2 = self.attack * self.env2 + (1 - self.attack) * self.env
        return self.env2

    def update(self):
        self.attack = exp(-2.0 * pi * 1000.0 / self.attack_input / self.sample_rate)
        self.release = exp(
            -2.0
            * pi
            * 1000.0
            / (self.release_input + self.attack_input)
            / self.sample_rate
        )

    def set_sample_rate(self, sample_rate):
        self.sample_rate = sample_rate
        self.update()

    def set_attack(self, attack):
        self.attack_input = attack
        self.update()

    def set_release(self, release):
        self.release_input = release
        self.update()


# pre_det = DecoupledPeakDetector(attack, release, sample_rate)
det = DecoupledPeakDetector(attack, release, sample_rate)

a = 100.0 + floor(((attack) / 1000.0) * sample_rate)
b = 1000.0 + floor(((release) / 1000.0) * sample_rate)

print(a)
print(b)

for i in range(1, 3500):

    if i == a:
        data2.append(-0.75)
    if i == b:
        data2.append(-0.75)

    # detector_input = min((i / 1000), 1.0)
    detector_input = lin_from_db(-12)
    if 100 < i < 1000:
        detector_input = lin_from_db(0.0)

    # data.append(detector_input)

    env = detector_input + attack_gain * (env - detector_input)
    env_db = db_from_lin(env)

    # env_db = db_from_lin(detector_input)

    # cv = basic(env, threshold, ratio)
    cv = env_db - reiss(env_db, db_from_lin(threshold), db_from_lin(knee), ratio)
    # cv = lin_from_db(cv)
    cv = lin_from_db(-det.process_smooth(cv))
    # data2.append(env)

    # cv = lin_from_db(-env)

    data2.append(-cv)


print(attack_gain)
plt.plot(data)
plt.plot(data2)
plt.show()

