#!/usr/bin/env python3

import argparse
import csv
import sys
import random

if __name__ == "__main__":
    nouns = [line.rstrip('\n') for line in open('words_alpha.txt')]

    argparser = argparse.ArgumentParser()
    argparser.add_argument("rows_num", type=int)
    args = argparser.parse_args()

    writer = csv.writer(sys.stdout, delimiter=';')

    for i in range(args.rows_num):
        url = "http://banners.com/banner{}.jpg".format(i)
        shows_amount = random.randint(1, 1000)
        categories_num = random.randint(1, 10)
        categories = random.sample(nouns, categories_num)
        writer.writerow([url, shows_amount] + categories)
