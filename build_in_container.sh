#!/bin/bash
docker build --network=host -t ferrite .
docker run --name ferrite ferrite
docker cp ferrite:/ferrite ferrite
docker rm /ferrite
