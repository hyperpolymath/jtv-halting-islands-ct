"""JtV (Julia the Viper) — proof-of-concept reference interpreter.

Two grammatically-sealed sublanguages:
  - jtv.data_ast / jtv.data_parser / jtv.data_eval   : the total DATA language
  - jtv.control_ast / jtv.control_parser / jtv.control_eval : the Turing-complete CONTROL language
  - jtv.diode : the structural validator enforcing the one-way bridge
"""
