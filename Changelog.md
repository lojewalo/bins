<a name="v2.0.0"></a>
## v2.0.0 (2017-06-11)


#### Features

*   threading
*   fedora pastebin support
*   debug mode (--debug/-d)
*   JSON output (--json/-j)

#### Bug Fixes

*   remove pastie, since it's dead

#### Breaking Changes

*   --service is gone (see --bin/-b)
*   --write is gone (see --output/-o)
*   --input is gone (use URL as first argument to download)
*   all files are downloaded by default (old behavior specified by --list-all/-L)



<a name="v1.2.0"></a>
## v1.2.0 (2016-07-16)


#### Features

*   include more information in --version and just version in --help ([9b18f037](9b18f037))
*   add feature information to version string ([ef41cc39](ef41cc39))
*   allow JSON output when uploading ([48225af3](48225af3))
*   add --json/-j to output JSON info in input mode ([274dba29](274dba29), closes [#28](28))

#### Bug Fixes

*   do not read metadata unless there is a size limit ([09e9be0a](09e9be0a))
*   update magic-sys to hopefully fix compilation on ARM ([9134730a](9134730a))
* **arguments:**  change --json help message and conflicts ([bebe5d2c](bebe5d2c))



<a name="v1.1.0"></a>
## v1.1.0 (2016-06-27)


#### Features

*   add --write/-w for write mode (003aebbb, closes #25)
*   add --number-lines/-e to number the lines of files in input mode (148af987, closes #20)
*   add a file type blacklist using libmagic (94644e84)
*   add a blacklist for file name patterns and size (454fd021, closes #15)
* **engines:**  add --name/-N for --message and stdin file names (0c41888c, closes #19)

#### Bug Fixes

*   use different file separator and fewer newlines (275e3a7d)
*   correct error message for invalid paths (89428d8b)
* **arguments:**  prevent invalid file names when using --name (fbbf4a91, closes #26)
* **build:**  remove unused variable (1cd2cc24)
* **engines:**
  *  make Index detection stricter (7da57681)
  *  verify URLs for each bin (664e0360, closes #23)
* **magic:**  use bit-width for size_t cast (a310c325)
* **write:**
  *  remove extraneous newline from output (9f44d3b4)
  *  sanitize path of file names, not output dir (ca60a4a9)



<a name="v1.0.0"></a>
## v1.0.0 (2016-06-22)




