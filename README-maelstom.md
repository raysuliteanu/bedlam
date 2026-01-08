# maelstrom

The `maelstrom` script assumes `lib/maelstrom.jar` exists, to avoid committing
it to source control. The `maelstrom.jar` can be built from source from
[jepsen-io/maelstrom](https://github.com/jepsen-io/maelstrom).

After cloning the repo, I did the following:

```sh
$ ./package.sh
$ cp target/maelstrom-0.2.5-SNAPSHOT-standalone.jar <target-dir>/lib/maelstrom.jar
```

The `maelstrom` script was also copied from the `jepsen-io/maelstrom` repo, from
`pkg/maelstrom`.
