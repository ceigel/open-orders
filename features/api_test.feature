Feature: Public server apis
  Scenario: Connect to server, query and validate time
    Given A request to public url /0/public/Time
    When I send it
    Then The server responds with status ok
    And The response has the correct time format

  Scenario: Connect to server, query and validate ticker information
    Given A request to public url /0/public/Ticker?pair=xbtusd
    When I send it
    Then The server responds with status ok
    And The response has the correct ticker format

  Scenario: Connect to server, authenticate and query open orders
    Given An authenticated request to private url /0/private/OpenOrders
    When I send it
    Then The server responds with status ok
    And The response has the correct orders format
