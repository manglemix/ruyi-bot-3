#!/bin/bash
#SBATCH --account=notchpeak-gpu
#SBATCH --partition=notchpeak-gpu
#SBATCH --qos=notchpeak-gpu
#SBATCH --time=02:00:00
#SBATCH --ntasks=4
#SBATCH --mem=8G
#SBATCH -o slurmjob-%j.out-%N
#SBATCH -e slurmjob-%j.err-%N
#SBATCH --gres=gpu

# module load cuda
module load python/3.13.5

source .venv/bin/activate


