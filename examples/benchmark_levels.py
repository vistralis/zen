#!/usr/bin/env python3
"""Benchmark all 5 scan levels (L0-L4) with profile simulations.

Compares results against pip list for accuracy validation.
Simulates cache hit/miss scenarios for profile benchmarking.
"""

import os
import json
import time
import subprocess
from pathlib import Path
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass, field
from concurrent.futures import ThreadPoolExecutor

ENVS_DIR = Path("/localdisk/envs")

@dataclass
class ScanResult:
    level: str
    duration_ms: float
    package_count: int
    sample: Dict[str, str] = field(default_factory=dict)


def get_site_packages(env_path: Path) -> Optional[Path]:
    """Find site-packages for an environment."""
    lib_path = env_path / "lib"
    if not lib_path.exists():
        return None
    for entry in lib_path.iterdir():
        if entry.name.startswith("python") and entry.is_dir():
            sp = entry / "site-packages"
            if sp.exists():
                return sp
    return None


# =============================================================================
# SCAN LEVELS
# =============================================================================

def l0_list_envs() -> List[str]:
    """L0: Just list environment names."""
    return [d.name for d in ENVS_DIR.iterdir() 
            if d.is_dir() and (d / "bin" / "python").exists()]


def l1_scan(env_path: Path) -> Dict[str, str]:
    """L1: Parse .dist-info directory names."""
    site_packages = get_site_packages(env_path)
    if not site_packages:
        return {}
    
    result = {}
    for entry in site_packages.iterdir():
        name = entry.name
        if name.endswith(".dist-info"):
            name_ver = name[:-10]
            parts = name_ver.rsplit("-", 1)
            if len(parts) == 2 and parts[1] and parts[1][0].isdigit():
                result[parts[0].lower().replace("_", "-")] = parts[1]
    return result


def l2_scan(env_path: Path) -> Dict[str, str]:
    """L2: Fixed 256-byte METADATA read."""
    site_packages = get_site_packages(env_path)
    if not site_packages:
        return {}
    
    result = {}
    for entry in site_packages.iterdir():
        if entry.name.endswith(".dist-info"):
            metadata = entry / "METADATA"
            if metadata.exists():
                with open(metadata, "rb") as f:
                    content = f.read(256).decode("utf-8", errors="ignore")
                    name, version = None, None
                    for line in content.split("\n")[:5]:
                        if line.startswith("Name: "):
                            name = line[6:].strip()
                        elif line.startswith("Version: "):
                            version = line[9:].strip()
                        if name and version:
                            break
                    if name and version:
                        result[name.lower()] = version
    return result


def l3_scan(env_path: Path) -> Dict[str, Tuple[str, str]]:
    """L3: Fixed 256-byte METADATA + INSTALLER."""
    site_packages = get_site_packages(env_path)
    if not site_packages:
        return {}
    
    result = {}
    for entry in site_packages.iterdir():
        if entry.name.endswith(".dist-info"):
            dist_info = entry
            metadata = dist_info / "METADATA"
            installer_path = dist_info / "INSTALLER"
            
            name, version, installer = None, None, None
            if metadata.exists():
                with open(metadata, "rb") as f:
                    content = f.read(256).decode("utf-8", errors="ignore")
                    for line in content.split("\n")[:5]:
                        if line.startswith("Name: "): name = line[6:].strip()
                        elif line.startswith("Version: "): version = line[9:].strip()
                        if name and version: break
            
            if installer_path.exists():
                installer = installer_path.read_text().strip()
            
            if name and version:
                result[name.lower()] = (version, installer or "unknown")
    return result


def l4_scan(env_path: Path) -> Dict[str, dict]:
    """L4: Full metadata including direct_url.json."""
    site_packages = get_site_packages(env_path)
    if not site_packages:
        return {}
    
    result = {}
    for entry in site_packages.iterdir():
        if entry.name.endswith(".dist-info"):
            dist_info = entry
            metadata = dist_info / "METADATA"
            installer_path = dist_info / "INSTALLER"
            direct_url = dist_info / "direct_url.json"
            
            pkg = {"version": None, "installer": None, "source": "pypi", "editable": False}
            
            if metadata.exists():
                with open(metadata, "r") as f:
                    name = None
                    for line in f:
                        if line.startswith("Name: "): name = line[6:].strip()
                        elif line.startswith("Version: "): pkg["version"] = line[9:].strip()
                        if name and pkg["version"]: break
            else:
                continue
            
            if installer_path.exists():
                pkg["installer"] = installer_path.read_text().strip()
            
            if direct_url.exists():
                try:
                    data = json.loads(direct_url.read_text())
                    if "vcs_info" in data:
                        pkg["source"] = "git"
                        pkg["commit"] = data["vcs_info"].get("commit_id", "")[:8]
                    if data.get("dir_info", {}).get("editable"):
                        pkg["editable"] = True
                        pkg["source"] = "local"
                except:
                    pass
            
            if name:
                result[name.lower()] = pkg
    return result


def pip_list(env_path: Path) -> Dict[str, str]:
    """Reference: pip list --format=json."""
    python = env_path / "bin" / "python"
    if not python.exists():
        return {}
    try:
        result = subprocess.run(
            [str(python), "-m", "pip", "list", "--format=json"],
            capture_output=True, text=True, timeout=30
        )
        if result.returncode == 0:
            return {p["name"].lower(): p["version"] for p in json.loads(result.stdout)}
    except:
        pass
    return {}


# =============================================================================
# BENCHMARKS
# =============================================================================

def benchmark_env(env_name: str, runs: int = 3) -> Dict[str, ScanResult]:
    """Benchmark all levels for a single environment."""
    env_path = ENVS_DIR / env_name
    results = {}
    
    # L1
    times = []
    for _ in range(runs):
        start = time.perf_counter()
        data = l1_scan(env_path)
        times.append((time.perf_counter() - start) * 1000)
    results["L1"] = ScanResult("L1", sum(times)/len(times), len(data), 
                               {k: data[k] for k in list(data)[:3]})
    
    # L2
    times = []
    for _ in range(runs):
        start = time.perf_counter()
        data = l2_scan(env_path)
        times.append((time.perf_counter() - start) * 1000)
    results["L2"] = ScanResult("L2", sum(times)/len(times), len(data),
                               {k: data[k] for k in list(data)[:3]})
    
    # L3
    times = []
    for _ in range(runs):
        start = time.perf_counter()
        data = l3_scan(env_path)
        times.append((time.perf_counter() - start) * 1000)
    results["L3"] = ScanResult("L3", sum(times)/len(times), len(data),
                               {k: v[0] for k, v in list(data.items())[:3]})
    
    # L4
    times = []
    for _ in range(runs):
        start = time.perf_counter()
        data = l4_scan(env_path)
        times.append((time.perf_counter() - start) * 1000)
    results["L4"] = ScanResult("L4", sum(times)/len(times), len(data),
                               {k: v["version"] for k, v in list(data.items())[:3]})
    
    # pip (reference, run once)
    start = time.perf_counter()
    data = pip_list(env_path)
    results["pip"] = ScanResult("pip", (time.perf_counter() - start) * 1000, len(data),
                                {k: data[k] for k in list(data)[:3]})
    
    return results


def simulate_profiles(envs: List[str]):
    """Simulate different profile strategies."""
    print("\n" + "=" * 80)
    print("PROFILE SIMULATIONS")
    print("=" * 80)
    
    # Simulate: cached = 80%, changed = 20%
    cached_envs = envs[:int(len(envs) * 0.8)]
    changed_envs = envs[int(len(envs) * 0.8):]
    
    print(f"\nScenario: {len(cached_envs)} cached, {len(changed_envs)} changed")
    
    # TURBO profile: L1 only, trust cache
    start = time.perf_counter()
    with ThreadPoolExecutor(max_workers=8) as ex:
        list(ex.map(lambda e: l1_scan(ENVS_DIR / e), envs))
    turbo_time = (time.perf_counter() - start) * 1000
    print(f"  TURBO   (L1 all):      {turbo_time:8.2f}ms")
    
    # FAST profile: L1 + L2 fallback for changed
    start = time.perf_counter()
    with ThreadPoolExecutor(max_workers=8) as ex:
        list(ex.map(lambda e: l1_scan(ENVS_DIR / e), cached_envs))
        list(ex.map(lambda e: l2_scan(ENVS_DIR / e), changed_envs))
    fast_time = (time.perf_counter() - start) * 1000
    print(f"  FAST    (L1+L2):       {fast_time:8.2f}ms")
    
    # BALANCED profile: L1 detect + L3 for changed
    start = time.perf_counter()
    with ThreadPoolExecutor(max_workers=8) as ex:
        list(ex.map(lambda e: l1_scan(ENVS_DIR / e), cached_envs))
        list(ex.map(lambda e: l3_scan(ENVS_DIR / e), changed_envs))
    balanced_time = (time.perf_counter() - start) * 1000
    print(f"  BALANCED (L1+L3):      {balanced_time:8.2f}ms")
    
    # ACCURATE profile: L3 always
    start = time.perf_counter()
    with ThreadPoolExecutor(max_workers=8) as ex:
        list(ex.map(lambda e: l3_scan(ENVS_DIR / e), envs))
    accurate_time = (time.perf_counter() - start) * 1000
    print(f"  ACCURATE (L3 all):     {accurate_time:8.2f}ms")
    
    # FULL profile: L4 always (default)
    start = time.perf_counter()
    with ThreadPoolExecutor(max_workers=8) as ex:
        list(ex.map(lambda e: l4_scan(ENVS_DIR / e), envs))
    full_time = (time.perf_counter() - start) * 1000
    print(f"  FULL    (L4 all):      {full_time:8.2f}ms  [DEFAULT]")


def validate_accuracy(env_name: str):
    """Compare L1-L4 vs pip list for accuracy."""
    print(f"\n=== Accuracy Check: {env_name} ===")
    env_path = ENVS_DIR / env_name
    
    pip_data = pip_list(env_path)
    l1_data = l1_scan(env_path)
    l2_data = l2_scan(env_path)
    l4_data = l4_scan(env_path)
    
    # Check key packages
    for pkg in ["torch", "numpy", "transformers", "diffusers"]:
        pip_ver = pip_data.get(pkg, "--")
        l1_ver = l1_data.get(pkg, "--")
        l2_ver = l2_data.get(pkg, "--")
        l4_ver = l4_data.get(pkg, {}).get("version", "--") if pkg in l4_data else "--"
        
        match = "✓" if l1_ver == l2_ver == l4_ver == pip_ver else "✗"
        print(f"  {pkg:15s}: pip={pip_ver:20s} L1={l1_ver:20s} L4={l4_ver:20s} {match}")


def main():
    print("=" * 80)
    print("ZEN SCAN LEVELS BENCHMARK (L0-L4)")
    print("=" * 80)
    
    # L0: List envs
    start = time.perf_counter()
    envs = l0_list_envs()
    l0_time = (time.perf_counter() - start) * 1000
    print(f"\nL0 (list envs): {l0_time:.2f}ms - Found {len(envs)} environments")
    
    # Select test envs
    test_envs = []
    for name in ["agentml", "athena", "ai_toolkit", "flux-klein-nvfp4"]:
        if name in envs:
            test_envs.append(name)
    if not test_envs:
        test_envs = envs[:4]
    
    print(f"\nBenchmarking {len(test_envs)} environments: {test_envs}")
    
    # Per-env benchmarks
    all_results = {}
    for env_name in test_envs:
        print(f"\n--- {env_name} ---")
        results = benchmark_env(env_name)
        all_results[env_name] = results
        for level, res in results.items():
            sample = ", ".join(f"{k}={v}" for k, v in list(res.sample.items())[:2])
            print(f"  {level:4s}: {res.duration_ms:8.2f}ms ({res.package_count:3d} pkgs) [{sample}]")
    
    # Summary
    print("\n" + "=" * 80)
    print("SUMMARY (avg across environments)")
    print("=" * 80)
    levels = ["L1", "L2", "L3", "L4", "pip"]
    for level in levels:
        avg = sum(all_results[e][level].duration_ms for e in all_results) / len(all_results)
        pkgs = sum(all_results[e][level].package_count for e in all_results) / len(all_results)
        print(f"  {level:4s}: {avg:8.2f}ms avg ({pkgs:.0f} pkgs)")
    
    # Speedups
    pip_avg = sum(all_results[e]["pip"].duration_ms for e in all_results) / len(all_results)
    print(f"\n[Speedups vs pip]")
    for level in ["L1", "L2", "L3", "L4"]:
        level_avg = sum(all_results[e][level].duration_ms for e in all_results) / len(all_results)
        speedup = pip_avg / level_avg if level_avg > 0 else float('inf')
        print(f"  {level}: {speedup:6.1f}x faster")
    
    # Profile simulations
    simulate_profiles(envs)
    
    # Accuracy validation
    if test_envs:
        validate_accuracy(test_envs[0])


if __name__ == "__main__":
    main()
