========
cred-man
========

Intro
-----

cred-man is a simple command-line password manager written in Rust.

Some features:

- Database is stored in JSON format
- DB is encrypted using AES-256 in GCM mode
- AES key is derived from password using Scrypt

Example
-------

.. code-block::

  $ cred-man
  Enter password: <...>
  > add example.com
    data key: username foo
    data key: password
      value for password: bar
    data key: 
  inserted 'example.com', now storing 1 keys
  > get example.com
  Timestamp: 2015-11-02 22:09:37
  Data:
  password: bar
  username: foo
  > find ex
  example.com
  > help
  Commands:
   help
   quit
   add
   get
   list
   find
   del
   dump
   import
   rename
   edit
  > quit
