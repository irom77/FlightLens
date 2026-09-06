# Synthetic verification fixtures

All backup files here are synthetic and contain no user data or credentials. The three tagged configurations exercise the same explicit values against separately bundled release schemas. They do not represent full firmware dumps, verified defaults, or usable aircraft settings.

`rate-vectors.json` contains 2,016 outputs from the upstream C rate functions: four models, four parameter combinations, two QuickRates expo modes, and 21 stick positions for each release. Generation compiles the functions without modifying their bodies, using float arithmetic and the corresponding fixed setpoint limits. The Rust differential test allows 0.02 degrees/second for floating-point differences.

SHA-256 of each upstream `src/main/fc/rc.c` used:

- 4.3.0: `9fc9d19371a35ea9017826506581049a9d20e2f9582239d11d5d005804883ea0`
- 4.4.0: `a08c07b9dc257d1f5cdcb28b3079a5746c8e38858e998ac47a1be17eb0003b13`
- 4.5.0: `340cfd8de82ba2de2c9d072460423716fbd3b7a5d3e4799cdabc325eaf8dcbb9`

Run `python3 tools/build_rate_vectors.py` only when intentionally regenerating source-derived fixtures. Ordinary tests are entirely offline.
