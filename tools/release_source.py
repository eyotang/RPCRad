"""Reuse native release artifacts only when application sources are unchanged."""
import json
import os
from pathlib import Path
import re
import subprocess

from release import PLATFORMS, REPOSITORY, version

RECHECK_FILES = {
    '.github/workflows/ci.yml', 'tools/wayland-smoke.py',
    'tools/release_source.py', 'tools/tests/test_release_source.py',
    'docs/INSTALL.md', 'docs/RELEASING.md',
}


def validate_source(run, jobs, tag, revision, changed):
    if (run.get('status') != 'completed' or run.get('event') != 'push'
            or run.get('path') != '.github/workflows/ci.yml'
            or run.get('head_branch') != tag or run.get('head_sha') != revision
            or run.get('repository', {}).get('full_name') != REPOSITORY
            or run.get('head_repository', {}).get('full_name') != REPOSITORY):
        raise ValueError('Artifacts must come from this repository and the exact release tag')
    if set(changed) - RECHECK_FILES:
        raise ValueError('Application or packaging sources changed; rebuild every native package')
    builds = [job for job in jobs if job['name'].startswith('build (')]
    if len(builds) != len(PLATFORMS):
        raise ValueError('Missing native platform checks')
    for platform in PLATFORMS:
        matches = [job for job in builds if f', {platform}, ' in job['name']]
        if len(matches) != 1 or matches[0].get('conclusion') != 'success':
            raise ValueError(f'Native checks did not pass: {platform}')


def api(path):
    return json.loads(subprocess.check_output(['gh', 'api', f'repos/{REPOSITORY}/{path}']))


def main():
    tag = 'v' + version()
    source_run = os.environ.get('REUSE_RUN', '').strip()
    if source_run:
        if not re.fullmatch(r'[0-9]+', source_run):
            raise ValueError('Run ID must be numeric')
        revision = subprocess.check_output(['git', 'rev-parse', f'{tag}^{{commit}}'], text=True).strip()
        changed = subprocess.check_output(['git', 'diff', '--name-only', tag, 'HEAD', '--'], text=True).splitlines()
        validate_source(api(f'actions/runs/{source_run}'),
                        api(f'actions/runs/{source_run}/jobs?per_page=100')['jobs'],
                        tag, revision, changed)
        print(f'Validated native artifacts from {tag} ({revision})')
    else:
        source_run = os.environ['GITHUB_RUN_ID']
    with Path(os.environ['GITHUB_OUTPUT']).open('a', encoding='utf-8') as output:
        output.write(f'tag={tag}\nrun_id={source_run}\n')


if __name__ == '__main__':
    main()
