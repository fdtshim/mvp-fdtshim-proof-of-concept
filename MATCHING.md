Matching rules
==============

`dmi-match`
-----------

The first node matching all DMI strings for the device is used.


`compatible`
------------

The rules are inspired from the ones within `depthcharge`.

The ambiant FDT compatible strings are kept in the given order, which will be used to rank in order of priority.

If more than one compatible string exists, the last string is assumed to be too generic, and removed.
It is generally the generic SoC name, and could cause wrong matches.

> NOTE: an improvement could be to add a list of generic names to remove instead.
> In some limited cases, it still matches too eagerly: i.e. `"...", "sochip,s3", "allwinner,sun8i-v3"`.

For this given representative ambiant FDT:

```
contoso,device-rev1-sku2  // Rank 0
contoso,device-rev1       // Rank 1
contoso,device            // Rank 2
socvendor,socmodel        // Rank 3; but removed from matching.
```

The FDT list would be tried in order, as usual.

Assuming `contoso,device` matches first, it is saved as a rank 2 match.
Meaning that any following dtb matching `contoso,device` wouldn't be saved or used.

Next, assuming `contoso,device-rev1` matches, the previous match is discarted, and this one saved as rank 1.

If at any point a rank 0 match is made, we can stop trying to match.

> This was deemed a *safe enough* method of operation by looking at the output of:
> 
> ```
>  $ cd .../u-boot/arch/arm/dts
>  $ for f in *.dts; do grep 'compatible\s*=\s*"' "$f" | head -n1 ; done | sort
> ```
