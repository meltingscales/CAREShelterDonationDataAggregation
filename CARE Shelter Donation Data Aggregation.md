# CARE Shelter Donation Data Aggregation 

- excel spreadsheet

- donations come in from:
    - qgive
    - paypal
    - square
    - checks or cash, entered manually to DonorSnap, some batch process
    - others, right now is no upload facility that we use to bring those donations from different sources into donorsnap.
    - shelterluv

- data:
    - name
    - cost
    - email
    - visit dates
    - customer ID
    - address

we want to make a program that...
    - take data from these various sources and transfer them into DonorSnap field names so we can upload them into DonorSnap
    - get current donors from [qgive, paypal, square, shelterluv] and load them into donorsnap

I need:
    0: A technical PoC from DonorSnap
    1: To see how DonorSnap accepts data, the documentation around that excel upload page
    1.1: An example spreadsheet with at least 1 row of data
    2: ONLY IF 1 is completed, a field-to-field mapping from Chuck, example:
        qgive.name -> donorsnap.name
        paypal.name -> donorsnap.name
        square.name -> donorsnap.name
        (...)
    3: Then we can start testing the auto-generated excel spreadsheet to DonorSnap

Deadline:
    10/12/2025

Bonus:
    Holly would like the names and emails of all of these donors in 1 place. As a byproduct of writing this automation, we'll just achieve that.

    ...

    Holly also needs analytics on all of our volunteers. Who attended what day, who's missed a day, who is scheduled.