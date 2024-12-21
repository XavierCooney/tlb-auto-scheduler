#!/usr/bin/python3

import subprocess, sys, re, argparse, shlex, os, concurrent.futures, json, tqdm, random, time, shutil, resource

argparser = argparse.ArgumentParser()
argparser.add_argument('config_dir')
argparser.add_argument('--controller', action='store_true')
argparser.add_argument('--runner', action='store_true')
args = argparser.parse_args()

CPUS = 2
ATTEMPTS = 2
# NUM_ROUNDS = '50000000'
# NUM_ROUNDS = 50_000_000
NUM_ROUNDS = 100_000_000


assert re.match(r'^[a-zA-Z.0-9]+$', args.config_dir)
DEST_DIR = '/home/cs1521/xavc/tlb-scheduler'

if not (args.controller or args.runner):
    HOST = 'cs1521-control'
    RSYNC_DEST_DIR = f'{HOST}:{DEST_DIR}'

    print('Syncing source and config...')
    subprocess.check_call([
        'rsync', '-a',
        'src', 'Cargo.toml', 'Cargo.lock', args.config_dir, 'run_remote.py',
        RSYNC_DEST_DIR
    ])

    print('Calling remote...')
    subprocess.check_call([
        'ssh', '-t', HOST,
        shlex.quote(os.path.join(DEST_DIR, 'run_remote.py')),
        '--controller', shlex.quote(args.config_dir)
    ])
elif args.controller:
    os.chdir(DEST_DIR)
    print('Building...')
    subprocess.check_call([
        # thanks comp6991
        '6991', 'cargo', 'build', '--release'
    ])

    print('Removing old outputs...')

    if os.path.exists('output'):
        shutil.rmtree('output/')

    # runner
    labs = {
        'bongo': 24,
        'tabla': 26,
        'lyre': 20,
        'viola': 18,
        'cello': 18,
        'flute': 25,
        'oboe': 25,
        'bugle': 18,
        'horn': 18,
        # 'sitar': 25,
        # 'kora': 24,
        # 'alto': 25,
        # 'bass': 25,
        # 'clavier': 21,
        # 'organ': 21,
        # 'piano': 18,
    }

    all_machines = []
    for lab_name, lab_suffixes in labs.items():
        if isinstance(lab_suffixes, int):
            lab_suffixes = map('{:02}'.format, range(lab_suffixes))
        for lab_suffix in lab_suffixes:
            all_machines.append(f'{lab_name}{lab_suffix}')

    def invoke_one_machine(machine):
        completed_proc = subprocess.run([
            'ssh',
            '-o', 'ControlMaster=auto',
            '-o', 'ControlPath=~/.ssh/.control-%r.%h.%p',
            '-o', 'ControlPersist=10m',
            '-o', 'ForwardAgent=no',
            '-o', 'StrictHostKeyChecking=no',
            '-o', 'UserKnownHostsFile=/dev/null',
            '-o', 'LogLevel=quiet',
            '-o', 'ForwardX11=no',
            '-o', 'ConnectTimeout=15',
            machine,
            shlex.quote(os.path.join(DEST_DIR, 'run_remote.py')),
            '--runner', shlex.quote(args.config_dir)
        ], encoding='utf-8', stdout=subprocess.PIPE, stderr=subprocess.PIPE)

        stdout_lines = completed_proc.stdout.strip().split('\n')
        if completed_proc.returncode == 0 and completed_proc.stderr == '' and stdout_lines[-1].startswith('{'):
            return {
                'success': True,
                'output': json.loads(stdout_lines[-1]),
                'machine': machine,
            }
        else:
            return {
                'success': False,
                'stderr': completed_proc.stderr,
                'stdout': completed_proc.stdout,
                'returncode': completed_proc.returncode,
                'machine': machine,
            }


    print('Invoking runners...')

    start_time = time.time()

    thread_executor = concurrent.futures.ThreadPoolExecutor(max_workers=360)
    futures = [thread_executor.submit(invoke_one_machine, machine) for machine in all_machines]

    total_failed = 0
    total_success = 0
    total_cpu_time = 0

    statuses = []
    for future in tqdm.tqdm(concurrent.futures.as_completed(futures), total=len(futures)):
        status = future.result()
        statuses.append(future.result())
        if status['success']:
            total_success += 1
            total_cpu_time += status['output']['time_spent']
        else:
            total_failed += 1

    wall_clock_time = time.time() - start_time

    print(f'Ran solver on {total_success} machines, {total_cpu_time:.2f} total CPU seconds spent solving across {ATTEMPTS * total_success} attempts')
    print(f'Took {wall_clock_time:.2f} wall clock seconds (parallelism = {total_cpu_time / wall_clock_time:.2f})')
    print(f'Failed on {total_failed} machines')

    with open('statuses.json', 'w') as statuses_out:
        json.dump(statuses, statuses_out, indent=4)
else:
    os.chdir(DEST_DIR)

    print('Runner!')
    output = subprocess.check_output([
        './target/release/tlb_auto_scheduler', args.config_dir,
        '--cpus', str(CPUS),
        '--num-rounds', str(NUM_ROUNDS),
        '--total-attempts', str(ATTEMPTS),
        '--start-seed', str(random.randint(0, 2**64 - 1)),
    ], encoding='utf-8')
    time_spent = resource.getrusage(resource.RUSAGE_CHILDREN).ru_utime

    print('good')
    print(json.dumps({
        # 'res': output,
        'time_spent': time_spent
    }))
