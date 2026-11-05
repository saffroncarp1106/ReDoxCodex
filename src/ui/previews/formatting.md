### pH calculation

For a hydrogen ion concentration of \(1.0\times10^{-3}\,\mathrm{mol/L}\):

\[
\mathrm{pH}=-\log_{10}[\mathrm{H}^{+}]=-\log_{10}(1.0\times10^{-3})=3.00
\]

The result is **pH = 3.00**. A fraction is typeset as \(\frac{[\mathrm{H}^{+}]}{1\,\mathrm{mol/L}}\).

```python
from math import log10

ph = -log10(1.0e-3)
print(f"pH = {ph:.2f}")
```

1. Equations retain powers and subscripts.
2. Code retains indentation and copying.
