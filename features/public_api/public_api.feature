Feature: Public server apis
  Scenario: Connect to server, query and validate time
    Given The api url /0/public/Time
    When I do a GET request to it
    Then The server responds with status ok
    And The response has the correct time format
